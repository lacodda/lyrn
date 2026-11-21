import { useRef, useState, type KeyboardEvent, type ReactNode } from 'react'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { Frame, ListDetail, ListDetailBack, Pane, useListDetailTurn } from '@/components/ui/frame'
import { RowButton, type RowButtonProps } from '@/components/ui/list-row'
import { PageHeader } from '@/components/ui/page-header'
import { SearchField } from '@/components/ui/search-field'

/*
 * ListDetailScreen.
 *
 * A screen of things and the one that is open: a searchable list beside what
 * is open in it, with the empty states, the keyboard and the narrow width a
 * list needs and every product used to draw for itself.
 *
 * Every product of the line has this screen at least once - works, releases,
 * versions, a queue - and the parts that took each of them longest are not
 * the ones on the stand. They are the three kinds of nothing (none yet, none
 * matching, none open), a keyboard that walks the list the way a mail client
 * does, and a phone, where the list and the open thing take turns and the
 * focus has to follow whichever one is on screen.
 *
 * A block, not a primitive: it lands in the product's `components/blocks` to
 * be read and changed. It is written to be used as it stands all the same -
 * the data and every word come in through props, so a product can keep it
 * unedited for as long as the screen is the usual one, and edit it the day
 * it is not.
 *
 * What it does not own is where the open thing lives. `selected` and
 * `onSelect` are the product's, because in a real product the open thing is
 * an address - `/works/w3` - and the router is the one that knows it.
 */

/** A row of the list, in RowButton's slots. */
export interface ListDetailRow {
  /** What identifies the row at a glance: an icon, a status dot, a cover. */
  start?: ReactNode
  title: ReactNode
  /** A second line under the title. */
  description?: ReactNode
  /** What is measured about the row: a date, a count. */
  end?: ReactNode
}

/** What an empty state says. */
export interface EmptyWords {
  title: ReactNode
  body?: ReactNode
}

/** Every word the screen shows, in the product's language. */
export interface ListDetailScreenWords {
  /** What the list is called - the name of its scroll region: "Works". */
  list: string
  /** The search field's name and placeholder: "Find a work". */
  search: string
  /** What clears the search, in the field and in the empty result: "Clear the search". */
  clear: string
  /** The way back to the list where only one of the two fits: "All works". */
  back: ReactNode
  /** The list has nothing in it yet. */
  nothingYet: EmptyWords
  /** What was typed matches nothing. */
  nothingFound: EmptyWords
  /** Nothing is open - said where there is room beside the list. */
  nothingOpen: EmptyWords
}

export interface ListDetailScreenProps<T> {
  /** The screen's title. */
  title: ReactNode
  /** One sentence under it. */
  description?: ReactNode
  /** What acts on the screen as a whole: New, Import. */
  actions?: ReactNode
  /** What the list holds, in the order it shows. */
  items: readonly T[]
  /** A stable key per item: its id. */
  itemKey: (item: T) => string
  /** How an item reads as a row. */
  row: (item: T) => ListDetailRow
  /** The key of what is open, or null. */
  selected: string | null
  /** Told what to open - or null, to go back to the list. */
  onSelect: (key: string | null) => void
  /** Whether an item answers to what was typed. Leave it out and there is no
   * search: a list of seven does not need one. */
  matches?: (item: T, query: string) => boolean
  /** The open item's title: the heading of its pane and the name of the
   * region that scrolls. Words, because a region is named in words. */
  detailTitle: (item: T) => string
  /** What acts on the open item, beside its title. */
  detailActions?: (item: T) => ReactNode
  /** The open item itself. */
  detail: (item: T) => ReactNode
  /** The one thing worth doing on an empty screen: "New work". */
  emptyAction?: ReactNode
  /** A row drawn as something other than a button - the product's link, so
   * a row can be opened in a new tab: `(work) => <Link to={…} />`. A click
   * is then the link's: the router opens it and `selected` follows the
   * address. `onSelect` is still told by the keyboard and the way back. */
  rowRender?: (item: T) => RowButtonProps['render']
  /** Every word the screen shows: the list's name, the search box, the way
   * back, and what each empty state says. Words are passed in so the product
   * can translate them. */
  words: ListDetailScreenWords
  className?: string
}

export function ListDetailScreen<T>({
  title,
  description,
  actions,
  items,
  itemKey,
  row,
  selected,
  onSelect,
  matches,
  detailTitle,
  detailActions,
  detail,
  emptyAction,
  rowRender,
  words,
  className,
}: ListDetailScreenProps<T>) {
  const [query, setQuery] = useState('')
  const listRef = useRef<HTMLDivElement>(null)
  const titleRef = useRef<HTMLHeadingElement>(null)
  const searchRef = useRef<HTMLInputElement>(null)

  const typed = query.trim()
  const visible = matches && typed !== '' ? items.filter((item) => matches(item, typed)) : items
  const open = selected === null ? undefined : items.find((item) => itemKey(item) === selected)

  // Where the focus goes when the list and the open thing trade places on a
  // narrow screen: the frame's rule, shared with every list beside a detail.
  const turn = useListDetailTurn({
    selected,
    title: titleRef,
    rowOf: (key) => listRef.current?.querySelector<HTMLElement>(`[data-row-key="${CSS.escape(key)}"]`) ?? null,
    fallback: searchRef,
  })

  const choose = (key: string | null) => {
    turn(key)
    onSelect(key)
  }

  /* Up and Down walk the list and open what they land on - the list beside
   * a reading pane, as a mail client has it. Home and End go to the ends.
   * Tab still leaves the list in one step. */
  const walk = (event: KeyboardEvent<HTMLDivElement>) => {
    const rows = [...(listRef.current?.querySelectorAll<HTMLElement>('[data-row-key]') ?? [])]
    if (rows.length === 0) return
    const at = rows.findIndex((element) => element === document.activeElement)
    const next =
      event.key === 'ArrowDown'
        ? Math.min(at + 1, rows.length - 1)
        : event.key === 'ArrowUp'
          ? Math.max(at - 1, 0)
          : event.key === 'Home'
            ? 0
            : event.key === 'End'
              ? rows.length - 1
              : null
    if (next === null || at === -1) return
    event.preventDefault()
    rows[next]!.focus()
    const key = rows[next]!.dataset.rowKey!
    if (key !== selected) choose(key)
  }

  const header = <PageHeader title={title} description={description} actions={actions} className="mb-0 w-full" />

  // Nothing at all: one empty state for the screen, not an empty list beside
  // an empty pane - two of them say the same thing twice.
  if (items.length === 0) {
    return (
      <Frame head={header} className={className}>
        <EmptyState title={words.nothingYet.title} body={words.nothingYet.body} action={emptyAction} className="flex-1" />
      </Frame>
    )
  }

  return (
    <Frame head={header} className={className}>
      <ListDetail
        open={open !== undefined}
        list={
          <Pane
            label={words.list}
            head={
              matches === undefined ? undefined : (
                <SearchField
                  ref={searchRef}
                  value={query}
                  onValueChange={setQuery}
                  placeholder={words.search}
                  aria-label={words.search}
                  clearLabel={words.clear}
                  className="w-full"
                />
              )
            }
            bodyClassName="p-1"
          >
            {visible.length === 0 ? (
              <EmptyState
                plain
                variant="filtered"
                title={words.nothingFound.title}
                body={words.nothingFound.body}
                action={
                  <Button size="sm" onClick={() => setQuery('')}>
                    {words.clear}
                  </Button>
                }
                className="p-2"
              />
            ) : (
              <div ref={listRef} onKeyDown={walk} className="flex flex-col">
                {visible.map((item) => {
                  const key = itemKey(item)
                  const { start, title: rowTitle, description: rowDescription, end } = row(item)
                  return (
                    <RowButton
                      key={key}
                      data-row-key={key}
                      selected={key === selected}
                      start={start}
                      description={rowDescription}
                      end={end}
                      render={rowRender?.(item)}
                      onClick={() => {
                        // A link opens itself; a button asks the product.
                        if (rowRender === undefined) choose(key)
                        else turn(key)
                      }}
                    >
                      {rowTitle}
                    </RowButton>
                  )
                })}
              </div>
            )}
          </Pane>
        }
        detail={
          open === undefined ? (
            // A dashed placeholder where the open thing will stand: that is
            // what a dashed border means everywhere else in the set.
            <EmptyState title={words.nothingOpen.title} body={words.nothingOpen.body} className="flex-1" />
          ) : (
            <Pane
              label={detailTitle(open)}
              head={
                <>
                  <ListDetailBack label={words.back} onClick={() => choose(null)} />
                  {/* `tabIndex={-1}`: where to put the focus once the list
                      has stepped aside, and not a stop on the way through. */}
                  <h2
                    ref={titleRef}
                    tabIndex={-1}
                    className="min-w-0 flex-1 truncate text-base font-semibold text-text outline-none"
                  >
                    {detailTitle(open)}
                  </h2>
                  {detailActions === undefined ? null : (
                    <div className="flex shrink-0 items-center gap-2">{detailActions(open)}</div>
                  )}
                </>
              }
              bodyClassName="p-4"
            >
              {detail(open)}
            </Pane>
          )
        }
      />
    </Frame>
  )
}
