// Holds every locale to the same shape as the source language.
//
// English is the source, never a fallback: a missing translation must fail the
// build rather than quietly render in English, because a half-translated screen
// is the kind of thing nobody reports and everybody notices.
//
//   node tools/check-locales.mjs
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

// fileURLToPath, not URL.pathname: on Windows the latter yields `/C:/...`,
// which joins into `C:\C:\...`.
const LOCALES = fileURLToPath(new URL('../src/i18n/locales/', import.meta.url))
const SOURCE = 'en'
const OTHERS = ['ru']

/** Every leaf as a dotted path, so two files can be compared key by key. */
function flatten(value, prefix = '') {
  const out = new Map()
  for (const [key, child] of Object.entries(value)) {
    const path = prefix === '' ? key : `${prefix}.${key}`
    if (child !== null && typeof child === 'object' && !Array.isArray(child)) {
      for (const [nested, leaf] of flatten(child, path)) out.set(nested, leaf)
    } else {
      out.set(path, child)
    }
  }
  return out
}

/** `{{name}}` and `{{count}}` - the parts a translation may not invent or drop. */
function placeholders(text) {
  if (typeof text !== 'string') return new Set()
  return new Set([...text.matchAll(/\{\{\s*([\w.]+)\s*\}\}/g)].map((match) => match[1]))
}

// i18next appends a plural category to the key; those are alternates of one
// message, not keys the other locale has to mirror one for one.
const PLURAL_SUFFIX = /_(zero|one|two|few|many|other)$/
const stem = (key) => key.replace(PLURAL_SUFFIX, '')

function read(locale) {
  try {
    return flatten(JSON.parse(readFileSync(join(LOCALES, `${locale}.json`), 'utf8')))
  } catch (cause) {
    console.error(`x ${locale}.json could not be read: ${cause.message}`)
    process.exit(1)
  }
}

const source = read(SOURCE)
let failed = false

for (const locale of OTHERS) {
  const target = read(locale)
  const stems = new Set([...target.keys()].map(stem))

  for (const [key, text] of source) {
    if (!stems.has(stem(key))) {
      console.error(`x ${locale}: missing \`${key}\``)
      failed = true
      continue
    }
    const expected = placeholders(text)
    const actual = placeholders(target.get(key) ?? '')
    for (const name of expected) {
      if (!actual.has(name) && target.has(key)) {
        console.error(`x ${locale}: \`${key}\` drops the {{${name}}} placeholder`)
        failed = true
      }
    }
  }

  for (const key of target.keys()) {
    if (!new Set([...source.keys()].map(stem)).has(stem(key))) {
      console.error(`x ${locale}: \`${key}\` has no counterpart in ${SOURCE}`)
      failed = true
    }
  }
}

if (failed) process.exit(1)
console.log(`locales: ${OTHERS.length + 1} in step with ${SOURCE}`)
