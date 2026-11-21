import { useId, type HTMLAttributes, type ReactNode } from 'react'
import { useRender } from '@base-ui/react/use-render'
import { cn, type HeadingLevel } from 'dowel-ui'

/*
 * The left-hand list of a screen with sections, and the heading each opens.
 *
 * A settings screen is the usual case: five or six sections, each its own
 * address so it can be linked to and the back button walks between them,
 * listed down the left with the current one tinted. Every product draws the
 * same column, and every product draws the active row a little differently -
 * which is exactly the drift a shared list exists to stop.
 *
 * The rows are the product's links, not the component's. `render` takes the
 * item and answers with the element to draw it as - a router's `NavLink`, an
 * `<a>`, whatever the product navigates with - and the component puts the
 * row's clothes, its icon and its `aria-current` on it. Without `render` a
 * row is a button and `onSelect` says which one was pressed, for a screen
 * whose sections are state rather than routes.
 *
 * `SectionHeading` is the other half: the title and the one-line hint above
 * a section's body, so the column and the page it opens are set in the same
 * type.
 */

export interface SectionNavItem {
  id: string
  label: ReactNode
  icon?: ReactNode
  /** A line under the label saying what the section holds - "Theme,
   * language, start screen" - so a reader chooses a section by what is in
   * it rather than by guessing from one word. At most two lines; past that
   * it is cut, because a column of rows that grow unevenly stops being a
   * list. */
  description?: ReactNode
}

export interface SectionNavProps extends Omit<HTMLAttributes<HTMLElement>, 'children' | 'onSelect'> {
  /** What the list is called: the caption above it, and the name a screen
   * reader gives the landmark. */
  label: string
  /** The sections, in order. Each row is a button, or the element `render` returns for it. */
  items: readonly SectionNavItem[]
  /** Which item is the current page. */
  activeId?: string
  /** The element a row is drawn as - `render={(item) => <NavLink to={…} />}`.
   * The row's props are merged onto it, the way Button takes a `render`. */
  render?: (item: SectionNavItem) => useRender.RenderProp
  /** Pressed, whatever the row is drawn as. */
  onSelect?: (id: string) => void
  /** Keep the caption as the landmark's name and take it off the screen. For
   * a screen whose own title already says it - a settings screen headed
   * "Settings" with a list captioned "Settings" names itself twice, which is
   * what kilna's did. */
  labelHidden?: boolean
  /** Where the caption stands in the outline: `2` by default, beside the
   * screen's title. */
  level?: HeadingLevel
}

export function SectionNav({
  label,
  items,
  activeId,
  render,
  onSelect,
  labelHidden = false,
  level = 2,
  className,
  ...props
}: SectionNavProps) {
  const captionId = useId()
  const Heading = `h${level}` as const
  return (
    <nav aria-labelledby={captionId} className={cn('flex flex-col gap-0.5', className)} {...props}>
      <Heading id={captionId} className={cn('caption px-2.5 pb-2', labelHidden && 'sr-only')}>
        {label}
      </Heading>
      {items.map((item) => (
        <SectionNavRow
          key={item.id}
          item={item}
          active={item.id === activeId}
          render={render?.(item)}
          onSelect={onSelect}
        />
      ))}
    </nav>
  )
}

function SectionNavRow({
  item,
  active,
  render,
  onSelect,
}: {
  item: SectionNavItem
  active: boolean
  render?: useRender.RenderProp
  onSelect?: (id: string) => void
}) {
  return useRender({
    render,
    defaultTagName: 'button',
    props: {
      ...(render === undefined ? { type: 'button' } : {}),
      // Named as the current page for a screen reader, which cannot see that
      // it is the tinted one.
      'aria-current': active ? 'page' : undefined,
      onClick: () => onSelect?.(item.id),
      // The whole label on hover, since the row may have cut it. Only a
      // string can be a title; a label that is an element says itself.
      title: typeof item.label === 'string' ? item.label : undefined,
      className: cn(
        'flex w-full gap-2.5 rounded-md px-2.5 py-1.5 text-left text-sm text-dim no-underline transition-colors',
        item.description === undefined ? 'items-center' : 'items-start',
        'hover:bg-soft hover:text-text [&_svg:not([class*=size-])]:size-4 [&_svg]:shrink-0',
        'focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-accent',
        active && 'bg-accent-soft text-text [&_svg]:text-accent',
      ),
      children: (
        <>
          {item.icon ? <span aria-hidden className="contents">{item.icon}</span> : null}
          {/* One line, truncated, like a NavRail entry. A section column is a
              fixed width, and a label that wraps makes one row taller than
              every other - which in a two-language product happens to one
              section and not the rest, so the column reads as ragged and the
              line height stops meaning anything. `min-w-0` because a flex
              child will not shrink below its content without it. */}
          {item.description === undefined ? (
            <span className="min-w-0 flex-1 truncate">{item.label}</span>
          ) : (
            <span className="flex min-w-0 flex-1 flex-col">
              <span className={cn('truncate', active && 'font-semibold')}>{item.label}</span>
              {/* `dim` at this size: `faint` measured below AA at 10px in
                  both themes (the same finding as ListRow's notes). */}
              <span className="line-clamp-2 text-2xs text-dim">{item.description}</span>
            </span>
          )}
        </>
      ),
    },
  })
}

export interface SectionHeadingProps extends Omit<HTMLAttributes<HTMLDivElement>, 'title'> {
  /** The open section's title, drawn as a heading. */
  title: ReactNode
  /** A line under the title that says what the section is for. */
  description?: ReactNode
  /** Where the open section's title stands in the outline: `2` by default. */
  level?: HeadingLevel
}

/** The title of the open section and the line under it. */
export function SectionHeading({ title, description, level = 2, className, ...props }: SectionHeadingProps) {
  const Heading = `h${level}` as const
  return (
    <div className={cn('mb-4 flex flex-col gap-1', className)} {...props}>
      <Heading className="text-base font-semibold text-text">{title}</Heading>
      {description ? <p className="text-sm text-dim">{description}</p> : null}
    </div>
  )
}
