import { useRef, type ReactNode } from 'react'
import { cn } from 'dowel-ui'
import { Frame, ListDetail, ListDetailBack, Pane, useListDetailTurn } from '@/components/ui/frame'
import { PageHeader } from '@/components/ui/page-header'
import { SectionNav, type SectionNavProps } from '@/components/ui/section-nav'

/*
 * SettingsScreen.
 *
 * Settings in sections: a column of destinations, each named with a line of
 * what is in it, beside the open section - and on a phone, the column first
 * and then the section, with the way back.
 *
 * Harvested from kilna, whose settings sat on one page until a version that
 * counted what was on it: the whole profile editor, a status check, export,
 * backup, import and a command to copy, in one scroll where nobody found the
 * one thing they came for. Each section has its own address now, so it can
 * be linked to and the back button walks between them, and the column says
 * what is in each one - "Theme, language, start screen" - so a section is
 * chosen by its contents rather than guessed from a word.
 *
 * The column is the rail's width, not a list's: it holds destinations, one
 * per line, like the app's own rail beside it. It does not scroll with the
 * section, and `sticky` could not give that - sticky travels with the page
 * until it reaches its offset, so a long section dragged the column up as far
 * as the title before pinning, which reads as a menu that scrolls.
 *
 * What is inside a section is the product's: `SettingsGroup` gives it the
 * shape the line's settings share - a caption, a line under it, and the
 * fields one under another at the width of a field.
 */

export interface SettingsSection {
  id: string
  /** Its name in the column and at the top of the section. */
  label: string
  /** What is in it, under the name in the column. */
  description?: ReactNode
  icon?: ReactNode
  /** The section itself: its groups of fields. */
  content: ReactNode
  /** A bar at the foot of the section - the Save for a section that keeps a
   * draft, in sight however far down the edit was. */
  foot?: ReactNode
}

/** Every word the screen shows, in the product's language. */
export interface SettingsScreenWords {
  /** What the column is called: the landmark's name, kept off the screen
   * where the title already says it - "Settings". */
  sections: string
  /** The way back to the column where only one of the two fits: "All settings". */
  back: ReactNode
}

export interface SettingsScreenProps {
  /** The screen's title. */
  title: ReactNode
  /** One sentence under the title. */
  description?: ReactNode
  /** The sections, in the order the column lists them. */
  sections: readonly SettingsSection[]
  /** The open section's id. Null opens the column on a narrow screen and the
   * first section where there is room for both. */
  active: string | null
  /** Told which section to open - or null, to go back to the column. */
  onSelect: (id: string | null) => void
  /** A section drawn as the product's link, so it has an address:
   * `(section) => <Link to={`/settings/${section.id}`} />`. A press is then
   * the link's; `onSelect` is still told on the way back. */
  sectionRender?: SectionNavProps['render']
  /** The column's name and the way back, in the product's language. */
  words: SettingsScreenWords
  className?: string
}

export function SettingsScreen({
  title,
  description,
  sections,
  active,
  onSelect,
  sectionRender,
  words,
  className,
}: SettingsScreenProps) {
  const titleRef = useRef<HTMLHeadingElement>(null)
  const columnRef = useRef<HTMLDivElement>(null)

  // Where there is room for both, something is always open: the first
  // section stands in for none, and the column says so.
  const shown = sections.find((section) => section.id === active) ?? sections[0]

  const turn = useListDetailTurn({
    selected: active,
    title: titleRef,
    // The column's rows, in the order of the sections.
    rowOf: (id) =>
      columnRef.current?.querySelectorAll<HTMLElement>('nav > :not(h2)')[
        sections.findIndex((section) => section.id === id)
      ] ?? null,
  })

  const choose = (id: string | null) => {
    turn(id)
    onSelect(id)
  }

  const header = <PageHeader title={title} description={description} className="mb-0 w-full" />

  if (shown === undefined) return <Frame head={header} className={className}>{null}</Frame>

  return (
    <Frame head={header} className={className}>
      <ListDetail
        open={active !== null}
        width="rail"
        list={
          <Pane label={words.sections} bodyClassName="p-1.5">
            <div ref={columnRef}>
              <SectionNav
                label={words.sections}
                labelHidden
                activeId={shown.id}
                items={sections.map(({ id, label, description: hint, icon }) => ({
                  id,
                  label,
                  description: hint,
                  icon,
                }))}
                render={sectionRender}
                onSelect={(id) => {
                  // A link opens itself; a button asks the product.
                  if (sectionRender === undefined) choose(id)
                  else turn(id)
                }}
              />
            </div>
          </Pane>
        }
        detail={
          <Pane
            label={shown.label}
            head={
              <>
                <ListDetailBack label={words.back} onClick={() => choose(null)} />
                {/* `tabIndex={-1}`: where the focus goes once the column has
                    stepped aside, and not a stop on the way through. */}
                <h2
                  ref={titleRef}
                  tabIndex={-1}
                  className="min-w-0 flex-1 truncate text-base font-semibold text-text outline-none"
                >
                  {shown.label}
                </h2>
              </>
            }
            foot={shown.foot}
            bodyClassName="flex flex-col gap-6 p-4"
          >
            {shown.content}
          </Pane>
        }
      />
    </Frame>
  )
}

export interface SettingsGroupProps {
  /** The group's caption: "Appearance", "Notifications". */
  title: ReactNode
  /** A line under it, when the caption does not say enough. */
  description?: ReactNode
  /** Its fields, one under another. */
  children: ReactNode
  className?: string
}

/**
 * A group of settings inside a section.
 *
 * One field under another at the width of a field, as the mockup lays a
 * settings group out: side by side, a segment and a select read as one
 * control of two halves, and a field the width of the pane asks the eye to
 * cross a monitor between a label and what it names.
 */
export function SettingsGroup({ title, description, children, className }: SettingsGroupProps) {
  return (
    <section className={cn('flex max-w-md flex-col gap-4', className)}>
      {/* A heading, not a caption: the fields under it carry captions of
          their own, and a group named in the same small capitals as its
          fields reads as one more field. */}
      <div className="flex flex-col gap-0.5">
        <h3 className="text-sm font-semibold text-text">{title}</h3>
        {description === undefined ? null : <p className="text-xs text-dim">{description}</p>}
      </div>
      {children}
    </section>
  )
}
