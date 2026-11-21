import type { HTMLAttributes, ReactNode } from 'react'
import { cva, type VariantProps } from 'class-variance-authority'
import { cn, type HeadingLevel } from 'dowel-ui'

/*
 * PageHeader.
 *
 * What a screen says it is, and the buttons that act on the whole of it: a
 * title, a line under it, and the actions at the far end of the same row.
 * Every screen in the line opens with these three and every one of them set
 * the title in a different size, because `text-lg font-semibold` is the kind
 * of thing nobody looks up.
 *
 * The title is an `h1`. It is the one heading a screen is entitled to: the
 * shell's bar names the application and the rail names the destinations, so
 * this is the top of the content's outline, and a reader jumping by heading
 * lands here. `Container` below is the other half - the measure the content
 * under it is read at.
 */

export interface PageHeaderProps extends Omit<HTMLAttributes<HTMLElement>, 'title'> {
  /** The screen's title, drawn as the page's heading. It is the one heading the screen is entitled to. */
  title: ReactNode
  /** The line under the title. One sentence: this is the screen's subtitle,
   * not its documentation. */
  description?: ReactNode
  /** Buttons that act on the screen as a whole - New, Export, a filter. They
   * sit at the far end of the title's row and wrap under it when there is no
   * width left, rather than squeezing the title into two words. */
  actions?: ReactNode
  /** A trail, a status, a tab strip: whatever belongs between the heading and
   * the content. Drawn under the whole row. */
  children?: ReactNode
  /** Where the title stands in the outline: `1` by default, the top of the
   * screen's content. A header over the open item of a list beside its
   * detail is one level down. */
  level?: HeadingLevel
}

export function PageHeader({
  title,
  description,
  actions,
  className,
  children,
  level = 1,
  ...props
}: PageHeaderProps) {
  const Heading = `h${level}` as const
  return (
    <header className={cn('mb-4 flex flex-col gap-3', className)} {...props}>
      {/* `flex-wrap` and `items-start`: the actions drop to their own line on
          a narrow screen instead of the title truncating. A title is what the
          screen is; a button can wait a row. */}
      <div className="flex flex-wrap items-start justify-between gap-x-4 gap-y-2">
        <div className="flex min-w-0 flex-col gap-1">
          <Heading className="text-xl font-semibold tracking-tight text-text">{title}</Heading>
          {description === undefined ? null : (
            <p className="text-sm text-dim">{description}</p>
          )}
        </div>
        {actions === undefined ? null : (
          <div className="flex shrink-0 flex-wrap items-center gap-2">{actions}</div>
        )}
      </div>
      {children}
    </header>
  )
}

export const containerVariants = cva('mx-auto w-full min-w-0', {
  variants: {
    /*
     * How wide the content is allowed to get.
     *
     * A measure rather than a taste: text at more than about ninety
     * characters a line loses the reader on the way back to the left margin,
     * and a form whose fields run the width of a 32-inch monitor asks the eye
     * to travel between a label and its input.
     */
    width: {
      /* Reading and forms: settings, an article, a dialog's worth of fields
       * given a whole screen. */
      prose: 'max-w-[68ch]',
      /* The usual screen: a list, a card, a dashboard's columns. */
      default: 'max-w-5xl',
      /* Tables and boards, which are read by scanning down a column rather
       * than across a line, and are the worse for being penned in. */
      wide: 'max-w-7xl',
      /* No ceiling. For a screen that IS the window - a canvas, a map, a
       * timeline that earns every pixel it is given. */
      full: '',
    },
  },
  defaultVariants: { width: 'default' },
})

export interface ContainerProps
  extends HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof containerVariants> {}

/**
 * The measure the screen's content is read at.
 *
 * Width only, and deliberately: the padding belongs to `Screen`, which knows
 * whether it is scrolling and therefore whether a scrollbar is about to take
 * ten pixels off the right. A container that also padded would double it.
 */
export function Container({ width, className, ...props }: ContainerProps) {
  return <div className={cn(containerVariants({ width }), className)} {...props} />
}
