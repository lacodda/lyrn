import { useEffect, useRef, type HTMLAttributes, type MouseEventHandler, type ReactNode, type Ref, type RefObject } from 'react'
import { cn } from 'dowel-ui'
import { Button } from './button'
import { ScrollArea } from './scroll-area'

/*
 * Frame, Pane and ListDetail.
 *
 * The geometry of a screen whose bottom edge is the window's: a column that
 * hands its height down and scrolls nowhere itself, a panel whose bars stand
 * while its body scrolls between them, and a list beside what is open in it.
 *
 * Harvested from kilna, where it is the frame every screen and every tab of a
 * card is laid out in. It exists there because the obvious way to build a
 * screen - a column that scrolls - is wrong in three ways at once, and an
 * audit found eleven screens built that way after a release had said none
 * were: the head went up with the content, a table's sticky header had
 * nothing to stick to, and a field at the foot of a tab drifted off the
 * bottom of the window with everything above it.
 *
 * So the column never scrolls. Only a part of the body does:
 *
 * - `Frame` - a screen's column. A head that stands, a body that takes the
 *   rest, sometimes a foot that stands too.
 * - `Pane` - a panel with a bar at the top, a body that scrolls, and a bar
 *   at the foot: an open note, a version being read, a list with its search.
 * - `ListDetail` - a list beside what is open in it. kilna drew six such
 *   screens with five widths and three gaps; there are two widths here, a
 *   list's and the rail's (`--spacing-list`, `--spacing-rail`), and one gap.
 *
 * The gap is ten pixels between the blocks of a screen, whichever of these
 * holds them - the mockup's, and kilna's on every screen.
 *
 * What kilna never needed is a narrow window: it is a desktop product with a
 * floor on its width. The products on the web have phones, and a list beside
 * its detail at 390px is two columns too narrow to read. Below 672px of its
 * own width - a container query, so a rail beside it counts - ListDetail
 * shows one of the two: the list until something is open, then the open
 * thing alone, with `ListDetailBack` in its head to return. That is how a
 * list and its detail work at that width everywhere else, from mail to
 * settings, and it is decided here once rather than by each screen - as is
 * where the focus goes when the two trade places (`useListDetailTurn`).
 */

export interface FrameProps extends Omit<HTMLAttributes<HTMLDivElement>, 'children'> {
  /** What stands above the body: a title, the filters, the screen's actions. */
  head?: ReactNode
  /** What stands under it: a composer, a status line. */
  foot?: ReactNode
  children: ReactNode
}

/*
 * A bar whose parts all draw nothing is not drawn either: `empty:hidden`.
 * A head is often a list of things each shown on a condition - a filter only
 * while there is something to filter, an action only for an owner - and when
 * every one of them renders nothing the bar was an empty box that still cost
 * the screen a gap of ten pixels above the body. kilna found it on 05.11 and
 * kept its own copy of this file until the set had it.
 */
const BAR = 'flex shrink-0 flex-wrap items-center gap-2 empty:hidden'

/** A screen's column. Hands its height to the body and scrolls nowhere. */
export function Frame({ head, foot, children, className, ...props }: FrameProps) {
  return (
    <div data-frame className={cn('flex min-h-0 min-w-0 flex-1 flex-col gap-2.5', className)} {...props}>
      {head === undefined ? null : <div className={BAR}>{head}</div>}
      {/* `min-h-0` and `min-w-0`: a flex child will not shrink below its
          content without them, and a body that does not shrink pushes the
          foot off the bottom of the window - the defect this exists for. */}
      <div className="flex min-h-0 min-w-0 flex-1 flex-col">{children}</div>
      {foot === undefined ? null : <div className={BAR}>{foot}</div>}
    </div>
  )
}

export interface PaneProps extends Omit<HTMLAttributes<HTMLElement>, 'children'> {
  /** What the body holds: the scroll region's name, which it needs because
   * it becomes a tab stop when it overflows - "Works", "Version 3". */
  label: string
  /** The bar across the top: a title and what acts on the open thing. */
  head?: ReactNode
  /** The bar across the foot: a status, tags, a button that adds a row. */
  foot?: ReactNode
  children: ReactNode
  /** The body's padding. A list's rows pad themselves; prose needs room. */
  bodyClassName?: string
  /** The scrolling element, for a caller that reads or sets the position. */
  viewportRef?: Ref<HTMLDivElement>
}

/** A panel whose bars stand and whose body scrolls between them. */
export function Pane({ label, head, foot, children, className, bodyClassName, viewportRef, ...props }: PaneProps) {
  return (
    <section
      data-pane
      className={cn(
        // On paper the body runs on rather than being cut at the pane's edge.
        'flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden rounded-lg border border-line bg-raise print:overflow-visible',
        className,
      )}
      {...props}
    >
      {head === undefined ? null : (
        <div className={cn(BAR, 'border-b border-line px-3 py-2')}>{head}</div>
      )}
      {/* `vertical`: a pane scrolls down, not across. Base UI sizes its
          content to `fit-content`, and without this a row that should
          truncate to the column widens it instead - kilna's queue ran a
          hundred pixels past a 264px column before it held the axis. */}
      <ScrollArea label={label} orientation="vertical" viewportRef={viewportRef} className="flex-1">
        {/* The content's own box, inside the one Base UI measures: the
            viewport's single child is Base UI's, so padding set on the
            viewport would reach nothing. */}
        <div className={bodyClassName}>{children}</div>
      </ScrollArea>
      {foot === undefined ? null : (
        <div className={cn(BAR, 'border-t border-line px-3 py-2')}>{foot}</div>
      )}
    </section>
  )
}

export interface ListDetailProps extends Omit<HTMLAttributes<HTMLDivElement>, 'children'> {
  /** The list, usually a `Pane` of `RowButton`s. */
  list: ReactNode
  /** What is open in it, or what says nothing is. */
  detail: ReactNode
  /**
   * Whether something in the list is open. Where there is room for both it
   * changes nothing; where there is not, it decides which of the two is
   * shown. Required, because a screen that forgot it would show a phone the
   * list forever and the detail never.
   */
  open: boolean
  /**
   * `list` for rows that carry a title and a line under it - versions,
   * works, notes, a queue. `rail` for a column of destinations, the width of
   * the app's own rail - the sections of the settings.
   */
  width?: 'list' | 'rail'
  /** Which side the list stands on. A queue read against a month stands
   * after it. */
  side?: 'start' | 'end'
}

/** A list beside what is open in it, each scrolling on its own - and one at
 * a time where there is no room for both. */
export function ListDetail({
  list,
  detail,
  open,
  width = 'list',
  side = 'start',
  className,
  ...props
}: ListDetailProps) {
  const column = (
    <div
      data-list
      className={cn(
        'flex min-h-0 shrink-0 flex-col',
        width === 'list' ? 'w-list' : 'w-rail',
        // Too narrow for both: the column takes the width while nothing is
        // open, and steps aside once something is.
        '@max-2xl/list-detail:w-full',
        open && '@max-2xl/list-detail:hidden',
      )}
    >
      {list}
    </div>
  )
  const opened = (
    <div
      data-detail
      className={cn('flex min-h-0 min-w-0 flex-1 flex-col', !open && '@max-2xl/list-detail:hidden')}
    >
      {detail}
    </div>
  )

  return (
    <div
      data-list-detail
      data-open={open ? '' : undefined}
      // The container the two columns measure: its own width, not the
      // window's, so a rail beside it is counted.
      className={cn('@container/list-detail flex min-h-0 min-w-0 flex-1 gap-2.5', className)}
      {...props}
    >
      {side === 'start' ? column : opened}
      {side === 'start' ? opened : column}
    </div>
  )
}

/** Whether an element is on screen: false once its column has stepped aside. */
function shown(element: Element | null | undefined): element is HTMLElement {
  if (!(element instanceof HTMLElement) || !element.isConnected) return false
  // `checkVisibility` is the browser's own answer, a hidden ancestor
  // included. Where it is missing, a laid-out element has boxes.
  return typeof element.checkVisibility === 'function'
    ? element.checkVisibility()
    : element.getClientRects().length > 0
}

export interface ListDetailTurnOptions {
  /** The key of what is open, or null. */
  selected: string | null
  /** The open thing's heading - where the focus goes once the list has
   * stepped aside. Give it `tabIndex={-1}`, or a browser will not focus it. */
  title: RefObject<HTMLElement | null>
  /** The row of a key, for the way back. */
  rowOf: (key: string) => HTMLElement | null
  /** Where the focus goes back to when the row that was open is gone - the
   * list's search field, say. */
  fallback?: RefObject<HTMLElement | null>
}

/**
 * Where the focus goes when the list and the open thing trade places.
 *
 * Beside each other nothing moves: the focus stays on the row that was
 * pressed, which is still there. Where only one fits, the pressed row has
 * just been hidden, and a browser drops the focus of a hidden element on the
 * floor - the next Tab starts from the top of the page, and a screen reader
 * says nothing at all. So it is handed to what took the row's place: the open
 * thing's heading, and on the way back the row that was open.
 *
 * Returns `turn`, to call with the key the reader asked for - before telling
 * the product. Only a turn the reader made moves the focus: a selection that
 * arrived from the address bar leaves it wherever the reader put it.
 */
export function useListDetailTurn({ selected, title, rowOf, fallback }: ListDetailTurnOptions) {
  const turned = useRef(false)
  // What was open before going back, so the reader lands on the row they
  // left rather than at the top of the list.
  const left = useRef<string | null>(null)

  useEffect(() => {
    if (!turned.current) return
    turned.current = false
    const focused = document.activeElement
    if (focused && focused !== document.body && shown(focused)) return
    if (selected !== null) {
      if (shown(title.current)) title.current.focus()
      return
    }
    const back = left.current === null ? null : rowOf(left.current)
    if (shown(back)) back.focus()
    else if (shown(fallback?.current)) fallback.current.focus()
  })

  return (key: string | null) => {
    turned.current = true
    if (key === null) left.current = selected
  }
}

export interface ListDetailBackProps {
  /** What it says: "All works", "Settings". The product's word, shown
   * beside the arrow. */
  label: ReactNode
  /** Called when the button is pressed. The screen uses it to close the open thing and show the list again. */
  onClick: MouseEventHandler<HTMLButtonElement>
  className?: string
}

/**
 * The way back to the list, for the width at which the list is not on
 * screen. Drawn only there: beside the list it would be a button that
 * returns to where the reader already is.
 */
export function ListDetailBack({ label, onClick, className }: ListDetailBackProps) {
  return (
    <Button variant="icon" size="sm" onClick={onClick} className={cn('@2xl/list-detail:hidden', className)}>
      <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.75" aria-hidden>
        <path d="M10 3.5L5.5 8l4.5 4.5" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
      {label}
    </Button>
  )
}
