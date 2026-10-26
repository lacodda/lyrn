// Copies what lyrn takes from dowel out of a published dowel-ui: the registry
// primitives the templates carry, and the line's accents.
//
//   node tools/vendor-dowel.mjs            # the version already vendored
//   node tools/vendor-dowel.mjs 0.34.0     # move to another version
//
// A generated project presses dowel's Button rather than a `<button>` - since
// dowel-ui 0.33 its lint says so - and its sign-in screen is made of Field,
// Input, Panel and Alert. Those are registry components: copied into a
// product, never imported from the package. So the template has to carry the
// copies, and `lyrn new` has to work offline, which rules out fetching them
// while generating.
//
// The copies are taken from the registry catalogue inside the npm package,
// the same file the generated project's `tools/check-registry.mjs` compares
// them against after `pnpm install`. A copy that does not match the dowel-ui
// the template pins fails that gate in lyrn's CI, and `tests` in lyrn fail
// when a template pins a dowel-ui other than the one vendored here - so moving
// the pin and forgetting this script is caught twice.
import { execFileSync } from 'node:child_process'
import { mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = fileURLToPath(new URL('../', import.meta.url))
const DOWEL = join(ROOT, 'src/templates/dowel')

/** The registry components some template writes. Adding a name here is half
 * the work; the other half is a `SourceFile` in the form that uses it. */
const PRIMITIVES = ['button', 'field', 'input', 'panel', 'alert']

/** The mark a project starts with: the line's umbrella mark, at all three
 * levels, until the product has one of its own. S is the favicon - the level
 * for 27px and under - and the three together are the masters in `assets/`
 * that `lyrn assets` draws every icon from. */
const MARKS = ['lacodda-L.svg', 'lacodda-M.svg', 'lacodda-S.svg']

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
  const wanted = new Set(['package/dist/registry.json', ...MARKS.map((mark) => `package/dist/marks/${mark}`), ...tiles])
  execFileSync('tar', ['-xzf', tarball, ...wanted], { cwd: scratch })

  const registry = JSON.parse(readFileSync(join(scratch, 'package/dist/registry.json'), 'utf8'))
  const byName = new Map(registry.items.map((item) => [item.name, item]))

  mkdirSync(join(DOWEL, 'ui'), { recursive: true })
  for (const name of readdirSync(join(DOWEL, 'ui'))) rmSync(join(DOWEL, 'ui', name))

  for (const name of PRIMITIVES) {
    const item = byName.get(name)
    if (!item) throw new Error(`dowel-ui ${version} has no registry item \`${name}\``)
    const missing = (item.registryDependencies ?? []).filter((dependency) => !PRIMITIVES.includes(dependency))
    if (missing.length > 0) {
      throw new Error(`\`${name}\` needs ${missing.join(', ')} - add them to PRIMITIVES`)
    }
    for (const file of item.files) {
      const target = join(DOWEL, 'ui', file.path.replace(/^.*\//, ''))
      writeFileSync(target, file.content)
      console.log(`${name}: ${file.path}`)
    }
  }
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
  for (const mark of MARKS) {
    writeFileSync(join(DOWEL, 'marks', mark), readFileSync(join(scratch, 'package/dist/marks', mark)))
  }
  console.log(`marks: ${MARKS.join(', ')}`)

  writeFileSync(join(DOWEL, 'VERSION'), `${version}\n`)
  console.log(`vendored ${PRIMITIVES.length} primitives from dowel-ui ${version}`)
} finally {
  rmSync(scratch, { recursive: true, force: true })
}
