// Copies what lyrn takes from dowel out of a published dowel-ui: the registry
// items the templates carry, the blocks `lyrn new --with block:<x>` writes,
// and the line's accents.
//
//   node tools/vendor-dowel.mjs            # the version already vendored
//   node tools/vendor-dowel.mjs 1.0.0      # move to another version
//
// A generated project presses dowel's Button rather than a `<button>` - since
// dowel-ui 0.33 its lint says so - and its sign-in screen is dowel's
// LoginScreen block. Those are registry items: copied into a product, never
// imported from the package. So the template has to carry the copies, and
// `lyrn new` has to work offline, which rules out fetching them while
// generating.
//
// The copies are taken from the registry catalogue inside the npm package,
// the same file the generated project's `tools/check-registry.mjs` compares
// them against after `pnpm install`. A copy that does not match the dowel-ui
// the template pins fails that gate in lyrn's CI, and `tests` in lyrn fail
// when a template pins a dowel-ui other than the one vendored here - so moving
// the pin and forgetting this script is caught twice.
//
// What is taken is a closure: the items named below and everything their
// `registryDependencies` reach, each written once. `catalogue.tsv` says, for
// each, what kind of item it is, where the registry puts it and what it
// needs - the part of the registry `lyrn new` resolves a block with, so the
// order of the copies is the registry's and not a list kept here by hand.
import { execFileSync } from 'node:child_process'
import { mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = fileURLToPath(new URL('../', import.meta.url))
const DOWEL = join(ROOT, 'src/templates/dowel')

/** The registry components some template writes. Adding a name here is half
 * the work; the other half is a `SourceFile` in the form that uses it. */
const PRIMITIVES = ['button', 'field', 'input', 'panel', 'alert', 'password-field']

/** Every block dowel publishes is one `--with block:<x>` away: a block is
 * taken whole, the day dowel has it, rather than the day it is listed here.
 * The names are read from the catalogue - every item of type
 * `registry:block` - and this is only what a block's name must look like. */
const BLOCK = /^[a-z0-9-]+-screen$/

/** `https://lacodda.github.io/dowel/r/<name>.json` is how an item names
 * another of the same registry. */
const dependencyName = (url) => url.match(/\/r\/([a-z0-9-]+)\.json$/)?.[1] ?? url

const version = process.argv[2] ?? readFileSync(join(DOWEL, 'VERSION'), 'utf8').trim()
if (!/^\d+\.\d+\.\d+$/.test(version)) {
  console.error(`not a version: ${version}`)
  process.exit(2)
}

const scratch = mkdtempSync(join(tmpdir(), 'lyrn-dowel-'))
try {
  // `npm pack` rather than the registry's HTTP API: it reads the tarball the
  // way `pnpm install` will, through whatever registry and proxy npm is set up
  // with. `shell` because npm is a `.cmd` on Windows.
  execFileSync('npm', ['pack', `dowel-ui@${version}`, '--pack-destination', scratch, '--silent'], {
    stdio: ['ignore', 'ignore', 'inherit'],
    shell: process.platform === 'win32',
  })
  const tarball = readdirSync(scratch).find((name) => name.endsWith('.tgz'))
  if (!tarball) throw new Error(`npm pack left no tarball in ${scratch}`)
  // Every product's S master, for the registry of marks: listed first and
  // then named, since GNU tar and Windows' bsdtar disagree about patterns.
  const listing = execFileSync('tar', ['-tzf', tarball], { cwd: scratch, encoding: 'utf8' })
  const tiles = listing.split(/\r?\n/).filter((path) => /^package\/dist\/marks\/[a-z0-9-]+-S\.svg$/.test(path))
  if (tiles.length === 0) throw new Error(`dowel-ui ${version} carries no marks in dist/marks`)
  // A relative archive name and `-C`: Windows' bsdtar reads `C:\...` as a
  // remote host, and GNU tar does the same with any colon in the path.
  // Every mark at all three levels: the umbrella mark a project starts with,
  // and the product's own for `lyrn new --accent <product>`. S is the favicon -
  // the level for 27px and under - and the three together are the masters in
  // `assets/` that `lyrn assets` draws every icon from.
  const masters = tiles.flatMap((tile) => ['L', 'M', 'S'].map((level) => tile.replace(/-S\.svg$/, `-${level}.svg`)))
  const wanted = new Set(['package/dist/registry.json', ...masters])
  execFileSync('tar', ['-xzf', tarball, ...wanted], { cwd: scratch })

  const registry = JSON.parse(readFileSync(join(scratch, 'package/dist/registry.json'), 'utf8'))
  const byName = new Map(registry.items.map((item) => [item.name, item]))

  const blocks = registry.items.filter((item) => item.type === 'registry:block').map((item) => item.name)
  for (const name of blocks) {
    if (!BLOCK.test(name)) throw new Error(`dowel-ui ${version}: the block \`${name}\` is not named <x>-screen`)
  }
  if (blocks.length === 0) throw new Error(`dowel-ui ${version} carries no blocks`)

  // The closure, dependencies before what needs them, each item once.
  const taken = []
  const seen = new Set()
  const take = (name, path) => {
    if (seen.has(name)) return
    if (path.includes(name)) throw new Error(`a cycle in the registry: ${[...path, name].join(' -> ')}`)
    const item = byName.get(name)
    if (!item) throw new Error(`dowel-ui ${version} has no registry item \`${name}\`, which ${path.at(-1) ?? 'lyrn'} needs`)
    for (const dependency of item.registryDependencies ?? []) take(dependencyName(dependency), [...path, name])
    seen.add(name)
    taken.push(item)
  }
  for (const name of [...PRIMITIVES, ...blocks]) take(name, [])

  for (const dir of ['ui', 'blocks']) {
    mkdirSync(join(DOWEL, dir), { recursive: true })
    for (const name of readdirSync(join(DOWEL, dir))) rmSync(join(DOWEL, dir, name))
  }

  // One line per item: its name, its kind, where the registry puts its one
  // file, what it needs of the registry and of npm.
  const catalogue = []
  for (const item of taken) {
    const kind = { 'registry:ui': 'ui', 'registry:block': 'block' }[item.type]
    if (!kind) throw new Error(`\`${item.name}\` is a ${item.type}, and lyrn carries only ui items and blocks`)
    if (item.files?.length !== 1) throw new Error(`\`${item.name}\` has ${item.files?.length ?? 0} files, and lyrn carries items of one`)
    const [file] = item.files
    const base = file.path.replace(/^.*\//, '')
    const dir = kind === 'ui' ? 'ui' : 'blocks'
    writeFileSync(join(DOWEL, dir, base), file.content)
    const needs = (item.registryDependencies ?? []).map(dependencyName).join(',')
    const packages = (item.dependencies ?? []).join(',')
    catalogue.push([item.name, kind, file.target ?? file.path, needs, packages].join('\t'))
    console.log(`${item.name}: ${dir}/${base}`)
  }
  writeFileSync(join(DOWEL, 'catalogue.tsv'), `${catalogue.join('\n')}\n`)
  console.log(`catalogue: ${taken.length} items, ${blocks.length} of them blocks`)
  // The line's products and their accents, from the same catalogue: dowel
  // publishes one `accent-<product>` item per mark in the brand registry, so
  // `--accent hilvan` knows hilvan the day dowel does - rather than the day
  // somebody remembers a second table inside lyrn.
  const accents = []
  for (const item of registry.items) {
    const product = item.name.match(/^accent-(.+)$/)?.[1]
    if (!product) continue
    const hex = item.files?.[0]?.content.match(/--accent-base:\s*(#[0-9a-fA-F]{6})/)?.[1]
    if (!hex) throw new Error(`\`${item.name}\` declares no --accent-base`)
    accents.push(`${product}\t${hex.toUpperCase()}`)
  }
  if (accents.length === 0) throw new Error(`dowel-ui ${version} carries no accent-* items`)
  accents.sort()
  writeFileSync(join(DOWEL, 'accents.tsv'), `${accents.join('\n')}\n`)
  console.log(`accents: ${accents.length} products`)

  // The registry of marks `lyrn brand pick` holds a new mark against: each
  // product's code and colour - or pair of colours, in the order the
  // gradient runs - read from the S master, which draws exactly those. A
  // product's first colour is its accent, and the two lists are held to that.
  const accentOf = new Map(accents.map((line) => line.split('\t')))
  const marks = []
  for (const tile of tiles) {
    const product = tile.match(/\/([a-z0-9-]+)-S\.svg$/)[1]
    const svg = readFileSync(join(scratch, tile), 'utf8')
    const code = svg.match(/<text[^>]*>([^<]+)<\/text>/)?.[1]
    const stops = [...svg.matchAll(/stop-color="(#[0-9a-fA-F]{6})"/g)].map((m) => m[1].toUpperCase())
    const fill = svg.match(/<polygon[^>]*fill="(#[0-9a-fA-F]{6})"/)?.[1]?.toUpperCase()
    const colours = stops.length > 0 ? stops : fill ? [fill] : []
    if (!code || colours.length === 0 || colours.length > 2) {
      throw new Error(`${tile}: cannot read a code and one or two colours from it`)
    }
    const accent = accentOf.get(product)
    if (accent && accent !== colours[0]) {
      throw new Error(`${product}: the accent is ${accent}, and its mark starts with ${colours[0]}`)
    }
    marks.push([product, code, ...colours].join('\t'))
  }
  marks.sort()
  writeFileSync(join(DOWEL, 'marks.tsv'), `${marks.join('\n')}\n`)
  console.log(`marks: ${marks.length} in the registry`)

  mkdirSync(join(DOWEL, 'marks'), { recursive: true })
  for (const master of masters) {
    writeFileSync(join(DOWEL, 'marks', master.replace(/^.*\//, '')), readFileSync(join(scratch, master)))
  }
  console.log(`masters: ${masters.length}, three for each mark`)

  writeFileSync(join(DOWEL, 'VERSION'), `${version}\n`)
  console.log(`vendored ${taken.length} registry items from dowel-ui ${version}`)
} finally {
  rmSync(scratch, { recursive: true, force: true })
}
