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

/** The favicon a project starts with: the line's umbrella mark at level S,
 * the level for 27px and under, until the product has a mark of its own. */
const FAVICON = 'lacodda-S.svg'

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
  // A relative archive name and `-C`: Windows' bsdtar reads `C:\...` as a
  // remote host, and GNU tar does the same with any colon in the path.
  execFileSync('tar', ['-xzf', tarball, 'package/dist/registry.json', `package/dist/marks/${FAVICON}`], { cwd: scratch })

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

  mkdirSync(join(DOWEL, 'marks'), { recursive: true })
  writeFileSync(join(DOWEL, 'marks', FAVICON), readFileSync(join(scratch, 'package/dist/marks', FAVICON)))
  console.log(`marks: ${FAVICON}`)

  writeFileSync(join(DOWEL, 'VERSION'), `${version}\n`)
  console.log(`vendored ${PRIMITIVES.length} primitives from dowel-ui ${version}`)
} finally {
  rmSync(scratch, { recursive: true, force: true })
}
