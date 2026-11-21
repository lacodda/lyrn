export {}
// Code first, so the description below survives `shadcn add`: the CLI writes
// a file from its first token on and drops every comment above it.

/*
 * Shortcut.
 *
 * The one notation the line writes keys in, and the one reading of a
 * keystroke against it, so that what is bound, what is shown and what a
 * person rebinds are the same words.
 *
 * A shortcut is a string, written the way it is read: `Mod+K`, `Mod+Shift+P`,
 * `?`, `Escape`. A sequence is its steps with a space between them - `G D` is
 * G, then D. A string because it has to live in more places than code: in a
 * settings file where someone has rebound it, in a menu hint, in the sheet that
 * lists every key, and in a comparison that says two commands share one.
 *
 * `Mod` is command on Apple platforms and control everywhere else. `Ctrl` is
 * the control key itself, which only means something different on a Mac -
 * `Ctrl+Tab` switches tabs there, where `Mod+Tab` belongs to the system.
 *
 * Two rules decide what a keystroke *is*, and both are about the people the
 * line is made for, who switch keyboard layouts in the middle of a sentence:
 *
 *   - A letter is the letter the key types when the layout types Latin, and
 *     the key's place when it does not. On a Russian layout `Ctrl+P` arrives
 *     as `з`; reading `event.key` there makes every shortcut in the
 *     application stop working the moment someone switches language to write.
 *   - Any other character is the character typed, with Shift inside it. `?`
 *     is Shift+/ on one keyboard and Shift+7 on another, and the person
 *     pressing it means the question mark on both. So `?` is written as `?`,
 *     never as `Shift+/` - and the notation refuses the second spelling,
 *     because two spellings of one key is how two commands end up on it
 *     without a conflict ever being seen.
 *
 * The number row under a command modifier is read by place as well: `Mod+1`
 * on a French keyboard is pressed on the key that types `&`.
 */

/** One step of a shortcut: a key and the modifiers held with it. */
export interface Stroke {
  /** An uppercase letter, a digit, a character such as `?`, or a key's name
   * such as `Escape` or `F5`. */
  key: string
  /** Command on Apple platforms, Control everywhere else. */
  mod: boolean
  /** The Control key on an Apple platform. Elsewhere Control *is* `Mod`. */
  ctrl: boolean
  alt: boolean
  /** Never set for a character: the character already carries it. */
  shift: boolean
}

/** Whether this machine writes and reads shortcuts the Apple way. */
export function isApplePlatform(): boolean {
  if (typeof navigator === 'undefined') return false
  return /mac|iphone|ipad|ipod/i.test(navigator.platform || navigator.userAgent)
}

/** Keys that have a name rather than a character, by the name `event.key`
 * gives them. Matched without regard to case, so `escape` is `Escape`. */
const NAMED = [
  'Escape',
  'Enter',
  'Tab',
  'Space',
  'Backspace',
  'Delete',
  'Insert',
  'Home',
  'End',
  'PageUp',
  'PageDown',
  'ArrowUp',
  'ArrowDown',
  'ArrowLeft',
  'ArrowRight',
  ...Array.from({ length: 24 }, (_, index) => `F${index + 1}`),
]
const NAMED_BY_LOWER = new Map(NAMED.map((name) => [name.toLowerCase(), name]))

/** The other spellings a person reaches for. Each turns into the one above,
 * so two spellings can never be two keys. */
const ALIASES: Record<string, string> = {
  esc: 'Escape',
  return: 'Enter',
  del: 'Delete',
  up: 'ArrowUp',
  down: 'ArrowDown',
  left: 'ArrowLeft',
  right: 'ArrowRight',
  plus: '+',
}

const MODIFIERS: Record<string, keyof Omit<Stroke, 'key'>> = {
  mod: 'mod',
  ctrl: 'ctrl',
  control: 'ctrl',
  alt: 'alt',
  option: 'alt',
  shift: 'shift',
}

/** What the punctuation keys type on a US layout, unshifted and shifted - the
 * place a key is read by when the layout typed a letter of its own there. */
const US_PUNCTUATION: Record<string, [string, string]> = {
  BracketLeft: ['[', '{'],
  BracketRight: [']', '}'],
  Semicolon: [';', ':'],
  Quote: ["'", '"'],
  Comma: [',', '<'],
  Period: ['.', '>'],
  Slash: ['/', '?'],
  Backslash: ['\\', '|'],
  Backquote: ['`', '~'],
  Minus: ['-', '_'],
  Equal: ['=', '+'],
}

/** Keys that are only ever held. Pressed alone they are a hand reaching for a
 * shortcut, not a shortcut. */
const HELD = new Set(['Control', 'Shift', 'Alt', 'Meta', 'AltGraph', 'OS', 'Hyper', 'Super', 'Fn', 'CapsLock'])

const isLetter = (key: string) => /^[A-Z]$/.test(key)
const isDigit = (key: string) => /^[0-9]$/.test(key)

/** A printable character other than a letter: it carries its own Shift. */
function isCharacter(key: string): boolean {
  return key.length === 1 && !isLetter(key)
}

/** Whether Mod, Ctrl or Alt is held - the modifiers that make a key a command
 * rather than something typed. Shift alone does not. */
export function holdsCommand(stroke: Stroke): boolean {
  return stroke.mod || stroke.ctrl || stroke.alt
}

/** Whether a digit is read by place in this stroke: under a command modifier.
 * Without one it is the character typed, Shift and all. */
const digitByPlace = (stroke: Stroke) => isDigit(stroke.key) && holdsCommand(stroke)

function parseStep(step: string, whole: string, apple: boolean): Stroke {
  // `+` is both the separator and a key, so `Mod++` and a lone `+` are the
  // plus key, and so is `Mod+Plus`.
  const tokens =
    step === '+' ? ['+'] : step.endsWith('++') ? [...step.slice(0, -2).split('+'), '+'] : step.split('+')
  const written = tokens.pop()!
  const stroke: Stroke = { key: '', mod: false, ctrl: false, alt: false, shift: false }

  for (const token of tokens) {
    const modifier = MODIFIERS[token.toLowerCase()]
    if (modifier === undefined) {
      throw new Error(
        `\`${token}\` in the shortcut \`${whole}\` is not a modifier. The line writes Mod, Ctrl, Alt and Shift - ` +
          'Mod is command on Apple platforms and control everywhere else, so there is no Cmd to write.',
      )
    }
    if (stroke[modifier]) throw new Error(`The shortcut \`${whole}\` names \`${token}\` twice.`)
    stroke[modifier] = true
  }

  const lower = written.toLowerCase()
  const named = NAMED_BY_LOWER.get(lower) ?? ALIASES[lower]
  if (named !== undefined) stroke.key = named
  else if (written.length === 1 && written >= '!' && written <= '~') stroke.key = written.toUpperCase()
  else {
    throw new Error(
      `\`${written}\` in the shortcut \`${whole}\` is not a key the line can read. Write a Latin letter, ` +
        'a digit, a character such as `?`, or a name such as `Escape` - a letter of another alphabet is ' +
        'found by its place on the keyboard, so it is written as the Latin letter it shares a key with.',
    )
  }

  if (stroke.shift && isCharacter(stroke.key) && !digitByPlace(stroke)) {
    throw new Error(
      `The shortcut \`${whole}\` holds Shift with \`${stroke.key}\`, and a character carries its own Shift: ` +
        'write the character it types instead - `?`, not `Shift+/`.',
    )
  }
  // Off an Apple platform the control key is the command key: `Ctrl+K` and
  // `Mod+K` are one keystroke there, and spelled one way they collide where
  // they should.
  if (!apple && stroke.ctrl) {
    stroke.ctrl = false
    stroke.mod = true
  }
  return stroke
}

/**
 * The steps of a shortcut. Throws on anything it cannot read, and says why:
 * a shortcut is written in code, and a typo in one is a key that silently
 * never fires.
 */
export function parseKeys(keys: string, apple: boolean = isApplePlatform()): Stroke[] {
  const steps = keys.trim().split(/\s+/)
  if (steps[0] === '') throw new Error('An empty shortcut binds nothing.')
  return steps.map((step) => parseStep(step, keys, apple))
}

/** The keys of one step in the one order: Mod, Ctrl, Alt, Shift, the key. */
export function strokeParts(stroke: Stroke): string[] {
  const parts: string[] = []
  if (stroke.mod) parts.push('Mod')
  if (stroke.ctrl) parts.push('Ctrl')
  if (stroke.alt) parts.push('Alt')
  if (stroke.shift) parts.push('Shift')
  parts.push(stroke.key)
  return parts
}

/** One step, written the one way. */
export function formatStroke(stroke: Stroke): string {
  return strokeParts(stroke).join('+')
}

/** What a key is called on this platform. `Mod` is the one that differs most:
 * command on Apple platforms, control everywhere else. */
export function keyLabel(key: string, apple: boolean = isApplePlatform()): string {
  const shared: Record<string, string> = {
    Enter: '↵',
    Escape: 'Esc',
    ArrowUp: '↑',
    ArrowDown: '↓',
    ArrowLeft: '←',
    ArrowRight: '→',
    Backspace: '⌫',
    Delete: 'Del',
    Tab: '⇥',
    Space: '␣',
    PageUp: 'PgUp',
    PageDown: 'PgDn',
  }
  const perPlatform: Record<string, [apple: string, other: string]> = {
    Mod: ['⌘', 'Ctrl'],
    Ctrl: ['⌃', 'Ctrl'],
    Alt: ['⌥', 'Alt'],
    Shift: ['⇧', 'Shift'],
  }

  const platform = perPlatform[key]
  if (platform) return apple ? platform[0] : platform[1]
  return shared[key] ?? key
}

/** A shortcut in its one spelling: `mod+shift+p` is `Mod+Shift+P`, `Esc` is
 * `Escape`. Two shortcuts are the same keys exactly when these agree. */
export function normalizeKeys(keys: string, apple: boolean = isApplePlatform()): string {
  return parseKeys(keys, apple).map(formatStroke).join(' ')
}

/** Whether two steps are the same keystroke. Exact on every modifier, in both
 * directions: `Mod+K` is not `Mod+Shift+K`, and a bare `K` is not `Ctrl+K`. */
export function sameStroke(a: Stroke, b: Stroke): boolean {
  return a.key === b.key && a.mod === b.mod && a.ctrl === b.ctrl && a.alt === b.alt && a.shift === b.shift
}

/** The key a keystroke pressed, by the two rules at the top of this file, and
 * whether Shift went into it. */
function readKey(event: KeyboardEvent, command: boolean): { key: string; character: boolean } | null {
  const { key } = event
  const code = event.code ?? ''

  if (/^[a-z]$/i.test(key)) return { key: key.toUpperCase(), character: false }
  // The number row under a command, by place: a French keyboard types `&`
  // on the key that `Mod+1` is pressed on.
  if (command && /^Digit[0-9]$/.test(code)) return { key: code.slice(5), character: false }
  if (key === ' ') return { key: 'Space', character: false }
  if (key.length === 1 && key >= '!' && key <= '~') return { key, character: true }

  const named = NAMED_BY_LOWER.get(key.toLowerCase())
  if (named !== undefined) return { key: named, character: false }

  // A layout that does not type Latin, or a dead key: the key's place.
  if (/^Key[A-Z]$/.test(code)) return { key: code.slice(3), character: false }
  if (command && /^Digit[0-9]$/.test(code)) return { key: code.slice(5), character: false }
  const punctuation = US_PUNCTUATION[code]
  if (punctuation) return { key: punctuation[event.shiftKey ? 1 : 0], character: true }
  return null
}

/**
 * What a keystroke is, as a step of a shortcut - or `null` when it is not one:
 * a modifier pressed alone, a key mid-composition in an input method, or
 * AltGr typing a character (Polish `ą` arrives as Ctrl+Alt+A on Windows, and
 * it is a letter, not a command).
 */
export function strokeOf(event: KeyboardEvent, apple: boolean = isApplePlatform()): Stroke | null {
  if (event.isComposing || event.keyCode === 229) return null
  if (HELD.has(event.key)) return null
  if (event.getModifierState?.('AltGraph')) return null

  let mod: boolean
  let ctrl: boolean
  if (apple) {
    mod = event.metaKey
    ctrl = event.ctrlKey
  } else {
    // The Windows key belongs to the system: nearly every chord on it is
    // taken before a page sees it, and the rest are not the page's to take.
    if (event.metaKey) return null
    mod = event.ctrlKey
    ctrl = false
  }
  const alt = event.altKey

  const read = readKey(event, mod || ctrl || alt)
  if (read === null) return null
  return { key: read.key, mod, ctrl, alt, shift: read.character ? false : event.shiftKey }
}

/** A keystroke written as a shortcut, for a settings screen that records one:
 * press the keys, store what this returns. */
export function keysOf(event: KeyboardEvent, apple: boolean = isApplePlatform()): string | null {
  const stroke = strokeOf(event, apple)
  return stroke === null ? null : formatStroke(stroke)
}

/**
 * Whether a field someone is typing in uses this key - a character, Space,
 * Enter, Tab, the arrows, Home and End, deleting.
 *
 * Only Escape and the function keys type nothing and move nothing, so they and
 * any step holding Mod, Ctrl or Alt are the only ones that can be allowed to
 * fire into a field.
 */
export function typesInField(stroke: Stroke): boolean {
  if (holdsCommand(stroke)) return false
  return stroke.key !== 'Escape' && !/^F[0-9]+$/.test(stroke.key)
}

/** Input types that hold no text: a keystroke on a checkbox is not typing. */
const NOT_TEXT = new Set(['checkbox', 'radio', 'button', 'submit', 'reset', 'range', 'color', 'file', 'image', 'hidden'])

/**
 * Whether the event landed somewhere that owns its own keys: a text input, a
 * textarea, a select, or anything editable - including an element *inside*
 * an editable one, which is what `target` reports for a bold word in an
 * editable paragraph.
 */
export function isTypingTarget(target: EventTarget | null): boolean {
  const element = target as Element | null
  if (!element || typeof element.closest !== 'function') return false
  // Asked first, because a browser also sets it for a document in design
  // mode, which no selector sees. An engine that does not lay out - jsdom, a
  // server render - leaves it `undefined`, and the selector below answers.
  if ((element as HTMLElement).isContentEditable) return true
  const field = element.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"])')
  if (field === null) return false
  if (field.tagName === 'INPUT') return !NOT_TEXT.has((field as HTMLInputElement).type)
  return true
}

/**
 * The shortcuts in `aria-keyshortcuts` form - `Control+K`, or `Meta+K` on a
 * Mac - for the button or menu item that does the same thing.
 *
 * A sequence has no form there: the attribute separates *alternatives* with a
 * space, so `G D` written into it would claim that G alone does it. Sequences
 * are left out, and a command with nothing else gets no attribute.
 */
export function ariaKeyShortcuts(
  keys: string | readonly string[],
  apple: boolean = isApplePlatform(),
): string | undefined {
  const written = (typeof keys === 'string' ? [keys] : keys)
    .map((written) => parseKeys(written, apple))
    .filter((steps) => steps.length === 1)
    .map(([stroke]) => {
      const parts: string[] = []
      if (stroke!.mod) parts.push(apple ? 'Meta' : 'Control')
      if (stroke!.ctrl) parts.push('Control')
      if (stroke!.alt) parts.push('Alt')
      if (stroke!.shift) parts.push('Shift')
      parts.push(stroke!.key === '+' ? 'Plus' : stroke!.key)
      return parts.join('+')
    })
  return written.length === 0 ? undefined : written.join(' ')
}
