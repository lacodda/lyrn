import type { HTMLAttributes, ReactNode } from 'react'
import { useRender } from '@base-ui/react/use-render'
import { cn } from 'dowel-ui'

/*
 * ListRow and RowButton.
 *
 * A row of a list, one thing said in a line: ListRow for a fact, RowButton for
 * a choice.
 *
 * Every screen of every product has a list of something - versions, releases,
 * a queue, a journal - and each drew its own row: kilna alone had six
 * list-and-detail layouts with five different row recipes, and the selected
 * row was tinted four different ways. The row is three slots and a rule
 * about truncation, which is small enough to get wrong every time and
 * exactly why it is here.
 *
 * Two components, because a row is either a fact or a choice:
 *
 * - `ListRow` is a fact - "Clip, 02.09" in a list of releases. It is not
 *   pressable, has no hover, and rows in a run are divided by a hairline.
 * - `RowButton` is a choice - a version in the list beside the editor, an
 *   item in a queue. It is a real `<button>` (or the product's link, through
 *   `render`), it answers the pointer, and the one that is open is tinted on
 *   `accent-soft` - the mockup's `.vrow.on` - and named as current, because
 *   the tint alone says it to nobody who cannot see it.
 *
 * The slots are the same in both: `start` for the thing that identifies the
 * row at a glance (an icon, a status dot, a cover), the row's words with an
 * optional `description` under them, and `end` for what is measured about it
 * (a date, a count, a chip). The words truncate on one line and the slots do
 * not shrink: a list whose rows grow to two lines in one language and not in
 * the other stops being a list you can scan.
 */

/* The quiet parts are `dim`, not `faint`. `--faint` is verified against the
 * grounds at body size, and at 11px under a row's title a live axe run
 * measured it below AA in both themes - on every list beside a detail, which
 * is every block that has a list. The title against them is `text`, so the
 * row still reads as a title and its notes. */
const slots = {
  start: 'flex shrink-0 items-center text-dim [&_svg:not([class*=size-])]:size-4',
  end: 'ml-auto flex shrink-0 items-center gap-1.5 text-xs text-dim tabular-nums',
} as const

interface RowSlots {
  /** What identifies the row at a glance: an icon, a status dot, a cover. */
  start?: ReactNode
  /** A second line under the words, quieter and truncated the same way. */
  description?: ReactNode
  /** What is measured about the row: a date, a count, a chip. */
  end?: ReactNode
  /** The row's words. */
  children: ReactNode
}

function RowBody({ start, description, end, children, strong }: RowSlots & { strong?: boolean }) {
  return (
    <>
      {start === undefined ? null : (
        <span data-slot="start" className={slots.start}>
          {start}
        </span>
      )}
      {/* `min-w-0` because a flex child refuses to shrink below its content
          without it - and a row that does not shrink pushes its `end` out of
          the list instead of truncating its words. */}
      <span className="flex min-w-0 flex-1 flex-col">
        <span className={cn('truncate', strong && 'font-semibold')}>{children}</span>
        {description === undefined ? null : <span className="truncate text-xs text-dim">{description}</span>}
      </span>
      {end === undefined ? null : <span className={slots.end}>{end}</span>}
    </>
  )
}

export interface ListRowProps extends RowSlots, Omit<HTMLAttributes<HTMLElement>, 'children'> {
  /** Something other than a `div` - an `<li>` in a `<ul>`, most often:
   * `render={<li />}`. */
  render?: useRender.RenderProp
}

/**
 * A row that states a fact. Rows in a run are divided by a hairline that the
 * last one does not draw, so a list inside a panel ends at the panel's edge
 * rather than on a line of its own.
 *
 * Its height comes from `py-row`, so a list follows `data-density` with the
 * table beside it.
 */
export function ListRow({ start, description, end, children, render, className, ...props }: ListRowProps) {
  return useRender({
    render,
    defaultTagName: 'div',
    props: {
      className: cn(
        'flex min-w-0 items-center gap-2.5 border-b border-line px-3 py-row text-sm text-text last:border-b-0',
        className,
      ),
      children: (
        <RowBody start={start} description={description} end={end}>
          {children}
        </RowBody>
      ),
      ...props,
    },
  })
}

export interface RowButtonProps extends RowSlots, Omit<HTMLAttributes<HTMLElement>, 'children'> {
  /** The open one - tinted, and named as current. */
  selected?: boolean
  /** The element to draw the row as - a router's link, most often:
   * `render={<NavLink to={…} />}`. Without it the row is a `<button>`. */
  render?: useRender.RenderProp
  /** Disables the row. Applied only when the row is the default `<button>`:
   * a rendered element such as a link has no `disabled` of its own. */
  disabled?: boolean
}

/**
 * A row that is a choice.
 *
 * `aria-current` says which one is open, the way a list beside an editor
 * works: the row is where you are, not a value you are submitting. A product
 * that puts rows in a `listbox` passes `aria-selected` instead (a toggle,
 * `aria-pressed`), and then `aria-current` is left off.
 */
export function RowButton({
  start,
  description,
  end,
  children,
  selected = false,
  render,
  disabled,
  className,
  ...props
}: RowButtonProps) {
  // The caller's own word for the state replaces `aria-current` rather than
  // joining it: a row in a listbox announced as both selected and current
  // reads one fact twice.
  const named = props['aria-selected'] !== undefined || props['aria-pressed'] !== undefined

  return useRender({
    render,
    defaultTagName: 'button',
    props: {
      // A `<button>` inside a form submits it unless told otherwise, and a
      // list of rows inside a settings form is where that bites.
      ...(render === undefined ? { type: 'button', disabled } : {}),
      'aria-current': selected && !named ? 'true' : undefined,
      className: cn(
        'flex w-full min-w-0 cursor-pointer items-center gap-2.5 rounded-md px-2.5 py-1.5 text-left text-sm text-text no-underline',
        'transition-colors hover:bg-soft',
        'focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent',
        'disabled:cursor-not-allowed disabled:opacity-50',
        // The open row's icon takes the accent, as a NavRail entry's does.
        selected && 'bg-accent-soft hover:bg-accent-soft [&>[data-slot=start]]:text-accent',
        className,
      ),
      children: (
        <RowBody start={start} description={description} end={end} strong>
          {children}
        </RowBody>
      ),
      ...props,
    },
  })
}
