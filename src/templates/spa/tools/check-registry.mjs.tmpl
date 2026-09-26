// Holds every dowel copy in `src/components/ui/` to the registry it came from.
//
// A lowercase file there is a copy of a dowel component and is not edited; a
// PascalCase one is this project's own. Without a check that is only a
// convention, and kilna found out what that is worth: a dozen copies had
// drifted - patched in place, or left behind by an upgrade - and nothing said
// so until somebody happened to run `dowel diff`. A copy that differs is either
// a fix dowel should have, or a copy of an older dowel than the one installed;
// neither should be found by accident.
//
// The registry is read from the installed package, so upgrading dowel-ui and
// not taking the copies again fails here. `pnpm exec dowel diff <name>` shows
// the lines.
//
// `public/favicon.svg` is held the same way: it is a copy of one of the line's
// marks, at level S - the one drawn for 27px and under. The larger levels
// turn to a smudge in a browser tab, and a mark drawn here by hand is one the
// brand registry has never seen.
//
//   node tools/check-registry.mjs
import { existsSync, readdirSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = fileURLToPath(new URL('../', import.meta.url))
const UI = join(ROOT, 'src/components/ui')
const PACKAGE = join(ROOT, 'node_modules/dowel-ui')

/** Copies allowed to differ, each with its reason. Empty is the goal: an entry
 * here is a debt with a name, not a way to make the check pass. */
const EXCEPTIONS = new Map()

const registry = JSON.parse(readFileSync(join(PACKAGE, 'dist/registry.json'), 'utf8'))
const { version } = JSON.parse(readFileSync(join(PACKAGE, 'package.json'), 'utf8'))
const upstream = new Map()
for (const item of registry.items) {
  for (const file of item.files ?? []) {
    upstream.set(file.path.replace(/^.*\//, ''), file.content)
  }
}

/** Line endings are the checkout's business, not the copy's. */
const normal = (text) => text.replace(/\r\n/g, '\n')

const drifted = []
const strays = []
let copies = 0

for (const name of existsSync(UI) ? readdirSync(UI).sort() : []) {
  if (!/\.tsx?$/.test(name) || /\.test\.tsx?$/.test(name)) continue
  // PascalCase is this project's own.
  if (!/^[a-z]/.test(name)) continue
  if (EXCEPTIONS.has(name)) continue
  const theirs = upstream.get(name)
  if (theirs === undefined) {
    strays.push(name)
    continue
  }
  copies += 1
  if (normal(readFileSync(join(UI, name), 'utf8')) !== normal(theirs)) drifted.push(name)
}

const problems = []
for (const name of drifted) {
  problems.push(
    `${name}: differs from dowel-ui ${version}` +
      ` - take the registry's copy (pnpm exec dowel diff ${name.replace(/\.tsx?$/, '')})`,
  )
}
for (const name of strays) {
  problems.push(
    `${name}: a lowercase file with no registry twin - a copy dowel no longer ships, or this project's own file named like a copy (use PascalCase)`,
  )
}

const FAVICON = join(ROOT, 'public/favicon.svg')
if (existsSync(FAVICON)) {
  const marks = join(PACKAGE, 'dist/marks')
  const ours = normal(readFileSync(FAVICON, 'utf8'))
  const twin = readdirSync(marks).find((name) => normal(readFileSync(join(marks, name), 'utf8')) === ours)
  if (twin === undefined) {
    problems.push(
      `public/favicon.svg: not one of the line's marks in dowel-ui ${version} - copy <product>-S.svg from node_modules/dowel-ui/dist/marks`,
    )
  } else if (!twin.endsWith('-S.svg')) {
    problems.push(`public/favicon.svg: is ${twin}, drawn for larger sizes - a favicon takes level S`)
  }
}

if (problems.length > 0) {
  for (const problem of problems) console.error(problem)
  process.exit(1)
}

console.log(`registry: ${copies} ${copies === 1 ? 'copy matches' : 'copies match'} dowel-ui ${version}`)
