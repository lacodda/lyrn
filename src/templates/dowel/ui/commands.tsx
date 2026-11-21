import { useLayoutEffect, useRef, useState, useSyncExternalStore, type ReactNode, type RefObject } from 'react'
import {
  formatStroke,
  holdsCommand,
  isTypingTarget,
  parseKeys,
  sameStroke,
  strokeOf,
  typesInField,
  type Stroke,
} from './shortcut'

/*
 * Commands.
 *
 * One list of everything the application can do, declared where the doing
 * lives - and the keyboard, the command palette and the sheet of shortcuts all
 * read that list, so a key is written once and none of them can be a version
 * behind.
 *
 * Every product that grew a palette wrote a key three times: in the listener
 * that answers it, in the palette row that hints at it, and in the help sheet
 * that lists it. The three drift - the sheet promises a key the listener no
 * longer answers, the palette hints one that was rebound - and nothing says
 * so. Here a command is declared once, `{ id, label, keys, run }`, by the
 * component that owns what it does, for as long as that component is mounted.
 *
 * There is one list per window, not one per provider, because there is one
 * keyboard per window. Two lists would be two listeners on the same keys,
 * which is exactly the conflict this exists to catch - so the list is the
 * module itself, and a command can be declared from anywhere without a
 * provider above it.
 *
 * What the list decides:
 *
 *   - **Sequences.** `G D` is G, then D within a second and a half. A key that
 *     does not continue the sequence ends it and does nothing else: a `g`
 *     pressed by mistake should not turn the next keystroke into a jump.
 *   - **Where a key is answered.** Not in a field someone is typing in, unless
 *     the command says it belongs there. A plain key - one without Mod, Ctrl
 *     or Alt - is not answered inside a dialog, a menu, a listbox or a tree
 *     either, which own their letters, nor Enter and Space on whatever has the
 *     focus, which they press. A command declared `within` an element answers
 *     only while the focus is inside it, and it wins over the rest of the
 *     application there.
 *   - **When a key is answered.** After whatever has the focus has had its
 *     turn, so a dialog's Escape and a list's arrows stay theirs - unless the
 *     command is declared `capture`, and takes the key first: an editor's own
 *     keymap would otherwise keep a key the person rebound onto it.
 *   - **Conflicts.** Two commands on the same keys in the same place, or one
 *     on a prefix of the other's sequence, cannot both work. The one declared
 *     first keeps the keys - mounted earlier, or earlier in the tree when
 *     mounted together, so a shell's own commands outrank a screen's - and the
 *     other loses them in the keyboard, in the palette's hint and in the sheet
 *     alike, and the console says which.
 */

export interface Command {
  /** Stable from one session to the next: a rebinding is stored against it. */
  id: string
  /** What the palette shows and the sheet lists, in the product's words. */
  label: string
  /** The heading it is listed under, in the palette and in the sheet. */
  group?: string
  /** Drawn at the start of its palette row. */
  icon?: ReactNode
  /** The keys, as `Mod+K` or `G D`. Several are alternatives. */
  keys?: string | readonly string[]
  /**
   * What it does. Leave it out for keys something else already answers -
   * Escape closing a dialog - which then appear in the sheet and take part in
   * the conflicts, but are not run from here.
   */
  run?: () => void
  /** Declared but not answering now. The sheet still lists its keys. */
  enabled?: boolean
  /**
   * Fire even in a field someone is typing in. For a key that cannot be
   * typing - one with Mod, Ctrl or Alt, Escape, a function key - and that
   * belongs wherever the person is: saving, the palette. A key a field uses
   * is refused when declared, because it would be taken from every field in
   * the application.
   */
  whileTyping?: boolean
  /** Answer only while the focus is inside this element. */
  within?: RefObject<Element | null>
  /**
   * Answer before whatever has the focus sees the key - in the capture phase,
   * rather than after it has had its turn. For a key the application keeps
   * even from an editor or a terminal with a keymap of its own: without it,
   * the editor answers any key its keymap binds and calls `preventDefault`,
   * and a command the person rebound onto that key is listed with it and does
   * nothing there.
   */
  capture?: boolean
  /**
   * Whether a list offers it to be run - the palette. Every command with
   * something to run is offered, except one declared `within`, whose work is
   * usually about where the focus is. `true` offers one whose key belongs in
   * one place but whose work does not need the focus there: folding in an
   * editor folds the editor the person was last in, wherever the focus is
   * now. `false` keeps one out - the command that opens the palette, which
   * would do nothing from inside it.
   */
  offered?: boolean
}

/** A command as the palette and the sheet read it. */
export interface ListedCommand {
  id: string
  label: string
  group?: string
  icon?: ReactNode
  /** The keys it answers to now, in the line's spelling: after a rebinding,
   * and without any it lost to a conflict. */
  keys: string[]
  /** Present when it can be run from a list - it has something to run, it is
   * enabled, and it is offered: not tied to where the focus is, or declared
   * to be offered anyway. The palette lists exactly these. */
  run?: () => void
}

/** Two commands on one set of keys, as the console reports it. */
export interface Conflict {
  /** The keys that were lost. */
  keys: string
  /** The command that lost them. */
  lost: string
  /** The keys that kept them: the same, or one a prefix of the other. */
  against: string
  /** The command that kept them. */
  kept: string
}

/** How long a sequence waits for its next key. Long enough not to hurry
 * anyone; short enough that a forgotten `g` cannot turn a later keystroke
 * into a jump. */
const SEQUENCE_WINDOW = 1500

/** Roles that own the plain keys pressed inside them: a dialog is a place of
 * its own, and a menu, a listbox or a tree types ahead. */
const OWNS_PLAIN_KEYS =
  '[role="dialog"], [role="alertdialog"], [role="menu"], [role="menubar"], [role="listbox"], [role="tree"], [role="treegrid"]'

interface Registration {
  commands: readonly Command[]
  /** What the lists show of these commands. When it has not changed, a new
   * render only hands over fresh handlers and nobody re-renders. */
  signature: string
}

interface Placed {
  token: number
  index: number
  command: Command
  /** The keys in the line's spelling. */
  keys: string
  steps: Stroke[]
  /** The element it answers inside, or `null` for the whole window. */
  scope: Element | null
}

const registrations = new Map<number, Registration>()
let nextToken = 0
let version = 0
let keymap: Record<string, readonly string[]> = {}
const listeners = new Set<() => void>()
const parsed = new Map<string, Stroke[]>()
const refIds = new WeakMap<object, number>()
let nextRefId = 0

let sorted: { version: number; entries: [number, Registration][] } = { version: -1, entries: [] }

/** The registrations in the order of the tree they were declared in. */
function ordered(): [number, Registration][] {
  if (sorted.version !== version) {
    sorted = { version, entries: [...registrations].sort(([left], [right]) => left - right) }
  }
  return sorted.entries
}

function stepsOf(keys: string): Stroke[] {
  let steps = parsed.get(keys)
  if (steps === undefined) {
    steps = parseKeys(keys)
    parsed.set(keys, steps)
  }
  return steps
}

/** The keys a command is declared with, or the ones it was rebound to. */
function declaredKeys(command: Command): readonly string[] {
  const rebound = keymap[command.id]
  if (rebound !== undefined) return rebound
  if (command.keys === undefined) return []
  return typeof command.keys === 'string' ? [command.keys] : command.keys
}

function refId(ref: object | undefined): number {
  if (ref === undefined) return -1
  let id = refIds.get(ref)
  if (id === undefined) {
    id = nextRefId++
    refIds.set(ref, id)
  }
  return id
}

function signatureOf(commands: readonly Command[]): string {
  return commands
    .map((command) =>
      [
        command.id,
        command.label,
        command.group ?? '',
        declaredKeys(command).join('|'),
        command.enabled === false ? 0 : 1,
        command.whileTyping ? 1 : 0,
        command.run ? 1 : 0,
        refId(command.within),
        command.capture ? 1 : 0,
        command.offered === undefined ? '' : command.offered ? 1 : 0,
      ].join('\u0000'),
    )
    .join('\u0001')
}

/** Refuses what cannot work, when it is declared rather than when it is
 * pressed: a key that does not parse never fires, and nobody would know. */
function validate(commands: readonly Command[]): void {
  for (const command of commands) {
    const keys = command.keys === undefined ? [] : typeof command.keys === 'string' ? [command.keys] : command.keys
    for (const written of keys) {
      const steps = stepsOf(written)
      if (command.whileTyping && steps.some(typesInField)) {
        throw new Error(
          `The command \`${command.id}\` fires while typing on \`${written}\`, which a field uses to type. ` +
            'Only a key with Mod, Ctrl or Alt, Escape or a function key can fire into a field.',
        )
      }
    }
  }
}

/** Whether the steps begin with what has been typed. */
function startsWith(steps: Stroke[], typed: Stroke[]): boolean {
  return steps.length >= typed.length && typed.every((stroke, index) => sameStroke(stroke, steps[index]!))
}

/** Every binding in the window, in the order declared, with the ones lost to
 * a conflict set aside. */
function place(): { kept: Placed[]; disabled: Placed[]; conflicts: Conflict[] } {
  const kept: Placed[] = []
  const disabled: Placed[] = []
  const conflicts: Conflict[] = []

  for (const [token, registration] of ordered()) {
    registration.commands.forEach((command, index) => {
      const scope = command.within === undefined ? null : command.within.current
      // Declared inside an element that is not on the page: there is nowhere
      // for it to answer.
      if (command.within !== undefined && scope === null) return

      for (const written of declaredKeys(command)) {
        const steps = stepsOf(written)
        const binding: Placed = { token, index, command, keys: steps.map(formatStroke).join(' '), steps, scope }
        if (command.enabled === false) {
          disabled.push(binding)
          continue
        }
        const rival = kept.find(
          (other) =>
            other.scope === scope && (startsWith(other.steps, binding.steps) || startsWith(binding.steps, other.steps)),
        )
        if (rival) {
          conflicts.push({ keys: binding.keys, lost: command.id, against: rival.keys, kept: rival.command.id })
          continue
        }
        kept.push(binding)
      }
    })
  }

  return { kept, disabled, conflicts }
}

interface Snapshot {
  version: number
  list: ListedCommand[]
  conflicts: Conflict[]
}

let snapshot: Snapshot = { version: -1, list: [], conflicts: [] }

function read(): Snapshot {
  if (snapshot.version === version) return snapshot
  const { kept, disabled, conflicts } = place()
  const keysOf = (token: number, index: number) =>
    [...kept, ...disabled].filter((binding) => binding.token === token && binding.index === index).map((binding) => binding.keys)

  const seen = new Set<string>()
  const list: ListedCommand[] = []
  for (const [token, registration] of ordered()) {
    registration.commands.forEach((command, index) => {
      // The same command declared by two mounted copies of one component is
      // one line in a list.
      if (seen.has(command.id)) return
      seen.add(command.id)
      const offered = command.offered ?? command.within === undefined
      const runnable = command.run !== undefined && command.enabled !== false && offered
      list.push({
        id: command.id,
        label: command.label,
        group: command.group,
        icon: command.icon,
        keys: keysOf(token, index),
        run: runnable ? () => registrations.get(token)?.commands[index]?.run?.() : undefined,
      })
    })
  }

  snapshot = { version, list, conflicts }
  return snapshot
}

let pending: { strokes: Stroke[]; timer: ReturnType<typeof setTimeout> } | null = null

function dropPending(): void {
  if (pending === null) return
  clearTimeout(pending.timer)
  pending = null
}

/** How deep an element sits, so the innermost place a key is declared in
 * answers it first. The whole window is shallower than anything in it. */
function depthOf(element: Element | null): number {
  let depth = -1
  for (let node = element; node !== null; node = node.parentElement) depth++
  return depth
}

/** Whether a binding answers this keystroke, landing where it landed. */
function answers(binding: Placed, target: Element | null, stroke: Stroke): boolean {
  const { command, scope } = binding
  if (scope !== null && (target === null || !scope.contains(target))) return false
  if (isTypingTarget(target) && (!command.whileTyping || typesInField(stroke))) return false

  if (!holdsCommand(stroke)) {
    const surface = target?.closest(OWNS_PLAIN_KEYS) ?? null
    // Unless the command was declared inside that very place.
    if (surface !== null && (scope === null || !surface.contains(scope))) return false
    const presses = stroke.key === 'Enter' || stroke.key === 'Space'
    const nothingFocused = target === null || target === document.body || target === document.documentElement
    if (presses && scope === null && !nothingFocused) return false
  }
  return true
}

/** What the list makes of the keys typed so far: the binding they finish, or
 * the bindings they begin, at the innermost place that has either. */
type Answer = { finished: Placed } | { begun: Placed[] }

function resolve(target: Element | null, typed: Stroke[], stroke: Stroke): Answer | null {
  const live = place().kept.filter(
    (binding) => binding.command.run !== undefined && answers(binding, target, stroke),
  )
  const depths = [...new Set(live.map((binding) => depthOf(binding.scope)))].sort((a, b) => b - a)

  for (const depth of depths) {
    const here = live.filter((binding) => depthOf(binding.scope) === depth)
    const exact = here.find((binding) => binding.steps.length === typed.length && startsWith(binding.steps, typed))
    if (exact) return { finished: exact }
    const begun = here.filter((binding) => binding.steps.length > typed.length && startsWith(binding.steps, typed))
    if (begun.length > 0) return { begun }
  }
  return null
}

/**
 * The keyboard, heard twice: in the capture phase, before whatever has the
 * focus, and in the bubble phase, after it.
 *
 * Both phases ask the same question of the same list, so where a key is
 * answered does not depend on the phase. What the phase decides is only when
 * the answer is acted on: a command declared `capture` takes the key on the
 * way down, before an editor's keymap can; every other command waits for the
 * way back up, so that something nearer - a dialog's Escape, a list's arrows
 * - keeps a key it handles.
 */
function onKeyDown(event: KeyboardEvent, capturing: boolean): void {
  if (event.defaultPrevented) return
  // A key held down while a sequence waits is the same key, not the next one.
  if (event.repeat && pending !== null) return
  const stroke = strokeOf(event)
  if (stroke === null) return

  const typed = pending === null ? [stroke] : [...pending.strokes, stroke]
  const answer = resolve(event.target instanceof Element ? event.target : null, typed, stroke)
  if (capturing) {
    const takes =
      answer !== null &&
      ('finished' in answer ? answer.finished.command.capture : answer.begun.some((binding) => binding.command.capture))
    // Not this command's phase: the sequence waits, untouched, for the other.
    if (!takes) return
  }
  dropPending()

  // Nothing answers. A key that ended a sequence is spent on ending it.
  if (answer === null) return
  event.preventDefault()
  if ('finished' in answer) {
    registrations.get(answer.finished.token)?.commands[answer.finished.index]?.run?.()
  } else {
    pending = { strokes: typed, timer: setTimeout(dropPending, SEQUENCE_WINDOW) }
  }
}

const onKeyDownCapturing = (event: KeyboardEvent) => onKeyDown(event, true)
const onKeyDownBubbling = (event: KeyboardEvent) => onKeyDown(event, false)

let listening = false
let reportScheduled = false
const reported = new Set<string>()

/** Says what lost its keys, once per conflict - after the commit settles, so
 * a strict-mode remount does not report the same thing twice. */
function scheduleReport(): void {
  if (reportScheduled) return
  reportScheduled = true
  queueMicrotask(() => {
    reportScheduled = false
    const current = new Set<string>()
    for (const conflict of read().conflicts) {
      const key = `${conflict.lost}\u0000${conflict.keys}\u0000${conflict.kept}\u0000${conflict.against}`
      current.add(key)
      if (reported.has(key)) continue
      console.error(
        `\`${conflict.lost}\` cannot answer to \`${conflict.keys}\`: \`${conflict.kept}\` already answers to ` +
          `\`${conflict.against}\` in the same place, and was declared first.`,
      )
    }
    reported.clear()
    for (const key of current) reported.add(key)
  })
}

function changed(): void {
  version++
  if (registrations.size > 0 && !listening && typeof document !== 'undefined') {
    document.addEventListener('keydown', onKeyDownCapturing, true)
    document.addEventListener('keydown', onKeyDownBubbling)
    listening = true
  } else if (registrations.size === 0 && listening) {
    document.removeEventListener('keydown', onKeyDownCapturing, true)
    document.removeEventListener('keydown', onKeyDownBubbling)
    listening = false
    dropPending()
  }
  for (const listener of listeners) listener()
  scheduleReport()
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

/**
 * Declare commands for as long as the calling component is mounted.
 *
 * The handlers may be written inline: each render hands over the newest ones,
 * and nothing is re-declared - and no list re-renders - unless something a
 * list shows has changed.
 */
export function useCommands(commands: readonly Command[]): void {
  const latest = useRef(commands)
  const token = useRef<number | null>(null)
  // The place in the list is taken at the first render, not when the effect
  // runs. Renders go in the order of the tree - a parent before its children,
  // siblings in turn - while effects run children first, which would list a
  // shell's own commands after everything inside it and hand a conflict to
  // whichever component happened to be deepest.
  const [place] = useState(() => nextToken++)

  useLayoutEffect(() => {
    latest.current = commands
    if (token.current === null) return
    const registration = registrations.get(token.current)
    if (registration === undefined) return
    const signature = signatureOf(commands)
    if (signature === registration.signature) {
      registration.commands = commands
      return
    }
    validate(commands)
    registrations.set(token.current, { commands, signature })
    changed()
  })

  useLayoutEffect(() => {
    const commands = latest.current
    validate(commands)
    registrations.set(place, { commands, signature: signatureOf(commands) })
    token.current = place
    changed()
    return () => {
      registrations.delete(place)
      token.current = null
      changed()
    }
  }, [place])
}

/** Declare one command for as long as the calling component is mounted. */
export function useCommand(command: Command): void {
  useCommands([command])
}

/** Every declared command, in the order declared, for the palette and the
 * sheet. Re-renders when what they show changes. */
export function useCommandList(): ListedCommand[] {
  return useSyncExternalStore(subscribe, () => read().list, () => read().list)
}

/** The keys one command answers to now - for the hint on a button that does
 * the same, or its `aria-keyshortcuts`. */
export function useCommandKeys(id: string): string[] {
  return useCommandList().find((command) => command.id === id)?.keys ?? []
}

/** The conflicts in the window now, for a settings screen that rebinds keys
 * to warn about before saving - or for a test that says there are none. */
export function useCommandConflicts(): Conflict[] {
  return useSyncExternalStore(subscribe, () => read().conflicts, () => read().conflicts)
}

/** Run a command by its id. False when nothing by that id can be run. */
export function runCommand(id: string): boolean {
  for (const [, registration] of ordered()) {
    const command = registration.commands.find((candidate) => candidate.id === id)
    if (command?.run !== undefined && command.enabled !== false) {
      command.run()
      return true
    }
  }
  return false
}

/**
 * Rebind keys: the person's changes, by command id, as the product stores
 * them. An empty list unbinds a command. Replaces whatever was set before.
 *
 * What is stored is the person's data rather than code, so a shortcut in it
 * that does not parse is reported and skipped rather than thrown: a settings
 * file written by an older version should not take the application down.
 */
export function setKeymap(rebound: Record<string, string | readonly string[]>): void {
  const next: Record<string, readonly string[]> = {}
  for (const [id, keys] of Object.entries(rebound)) {
    const list = typeof keys === 'string' ? [keys] : keys
    try {
      list.forEach(stepsOf)
      next[id] = list
    } catch (error) {
      console.error(`The keys rebound for \`${id}\` were skipped: ${(error as Error).message}`)
    }
  }
  keymap = next
  changed()
}

/** A label cut into words, whatever separates them. */
const words = (text: string) => text.split(/[^\p{L}\p{N}]+/u).filter((word) => word !== '')

/** How well a label answers the terms, best first; `null` when it does not. */
function rank(label: string, terms: string[]): number | null {
  const text = label.toLowerCase()
  if (!terms.every((term) => text.includes(term))) return null
  if (text.startsWith(terms.join(' '))) return 0
  const starts = words(text)
  if (terms.every((term) => starts.some((word) => word.startsWith(term)))) return 1
  return 2
}

/**
 * The commands whose label answers the query, best first.
 *
 * Every term has to be in the label, in any order - "theme dark" finds
 * "Switch to the dark theme". A label that begins with the query comes first,
 * one where every term begins a word next, one that merely contains the
 * letters last: "cal" is the Calendar long before it is "Critical". The sort
 * is stable, so equally good matches keep their order and the list does not
 * shuffle under the cursor as the query grows. An empty query matches all.
 */
export function matchCommands<T extends { label: string }>(query: string, commands: readonly T[]): T[] {
  const terms = words(query.toLowerCase())
  if (terms.length === 0) return [...commands]
  return commands
    .map((command) => ({ command, rank: rank(command.label, terms) }))
    .filter((entry): entry is { command: T; rank: number } => entry.rank !== null)
    .sort((left, right) => left.rank - right.rank)
    .map((entry) => entry.command)
}

/** Commands under their headings, in the order the headings first appear. */
export function groupCommands<T extends { group?: string }>(
  commands: readonly T[],
): { group: string | undefined; commands: T[] }[] {
  const groups: { group: string | undefined; commands: T[] }[] = []
  for (const command of commands) {
    const found = groups.find((entry) => entry.group === command.group)
    if (found) found.commands.push(command)
    else groups.push({ group: command.group, commands: [command] })
  }
  return groups
}
