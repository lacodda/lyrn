// Holds every package this project ships to the licenses it accepts.
//
//   node tools/check-licenses.mjs
//
// Production dependencies only: they are what the build puts in front of the
// people who use it. The list is the line's, the same one `deny.toml` holds
// Rust crates to - permissive licenses, and MPL-2.0, whose copyleft stops at
// the file it is in. A package under anything else is a decision rather than
// a dependency to add quietly: allow its license here and say why, or choose
// another package. A package with no license at all is reported the same way.
import { execFileSync } from 'node:child_process'

const ALLOWED = new Set([
  '0BSD',
  'Apache-2.0',
  'Apache-2.0 WITH LLVM-exception',
  'BSD-2-Clause',
  'BSD-3-Clause',
  'BSL-1.0',
  'BlueOak-1.0.0',
  'CC0-1.0',
  'CDLA-Permissive-2.0',
  'ISC',
  'MIT',
  'MIT-0',
  'MPL-2.0',
  'Unicode-3.0',
  'Unlicense',
  'Zlib',
])

/** Whether an SPDX expression is satisfied by the accepted licenses: one side
 * of an OR is enough, every side of an AND is needed, and parentheses group as
 * written. A bare split on `OR` would let `(A OR B) AND C` through on A alone. */
function accepted(expression) {
  const tokens = expression.match(/\(|\)|[^\s()]+/g) ?? []
  let at = 0
  const peek = () => tokens[at]
  const next = () => tokens[at++]

  function either() {
    let ok = both()
    while (peek() === 'OR') {
      next()
      ok = both() || ok
    }
    return ok
  }
  function both() {
    let ok = one()
    while (peek() === 'AND') {
      next()
      ok = one() && ok
    }
    return ok
  }
  function one() {
    if (peek() === '(') {
      next()
      const ok = either()
      if (next() !== ')') throw new Error(`unbalanced parentheses in "${expression}"`)
      return ok
    }
    let id = next()
    if (id === undefined) throw new Error(`incomplete license expression "${expression}"`)
    if (peek() === 'WITH') {
      next()
      id = `${id} WITH ${next()}`
    }
    return ALLOWED.has(id)
  }

  const ok = either()
  if (at !== tokens.length) throw new Error(`cannot read the license expression "${expression}"`)
  return ok
}

function main() {
  // `shell` because pnpm is a `.cmd` on Windows.
  const output = execFileSync('pnpm', ['licenses', 'list', '--prod', '--json'], {
    encoding: 'utf8',
    shell: process.platform === 'win32',
  })
  const byLicense = JSON.parse(output)

  const refused = []
  for (const [license, packages] of Object.entries(byLicense)) {
    if (accepted(license)) continue
    for (const pkg of packages) {
      refused.push(`${pkg.name}@${pkg.versions.join(', ')}: ${license}`)
    }
  }

  if (refused.length > 0) {
    console.error('Packages under a license this project has not accepted:')
    for (const line of refused) console.error(`  ${line}`)
    console.error('Accept the license in tools/check-licenses.mjs, with the reason, or choose another package.')
    process.exit(1)
  }
  const count = Object.values(byLicense).reduce((sum, packages) => sum + packages.length, 0)
  console.log(
    count === 1
      ? '1 production package, under an accepted license'
      : `${count} production packages, every one under an accepted license`,
  )
}

main()
