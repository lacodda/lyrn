import type { HTMLAttributes, ReactNode } from 'react'
import { cva, type VariantProps } from 'class-variance-authority'
import { cn } from 'dowel-ui'

/*
 * What a screen says when there is nothing on it.
 *
 * Three kinds of nothing, and a product that draws the same panel for all
 * three is telling the reader the wrong thing twice:
 *
 *   **empty** - there is nothing here yet, and that is normal. The panel says
 *   what would be here and offers the one action that makes it appear.
 *
 *   **filtered** - there is plenty here, just none of it matching. The way out
 *   is to widen the filter, not to create anything.
 *
 *   **error** - it could not be fetched. Nothing is missing; something failed,
 *   and the way out is to try again.
 *
 * They differ in what the reader should do next, which is exactly what an
 * empty screen is for, so they are a variant rather than three components.
 *
 * `action` is the whole point of the component: **an empty screen with no way
 * out is a dead end.** It is optional in the type because a panel inside a
 * larger screen can borrow the way out from its surroundings - but a full-page
 * empty state without one is a bug the reader cannot report.
 *
 * The mark is the line's hexagon, drawn in the current text colour rather than
 * a product's accent. A full-strength logo in an empty panel shouts; this is a
 * watermark, and it is `aria-hidden` because it says nothing a reader needs.
 *
 * **`plain` is the same three kinds with no frame.** The framed panel is for
 * a screen or a region that is empty; inside something that already has a
 * frame - a widget, a card, a panel's list - a dashed box and a watermark are
 * a frame inside a frame, and kilna drew seven such empties as bare text
 * instead, without the way out. Plain keeps the words and the action and
 * drops the rest: no border, no mark, set in the flow on the left, where the
 * content it stands in for would have started.
 */

export const emptyStateVariants = cva('flex flex-col gap-3', {
  variants: {
    variant: {
      empty: '',
      filtered: '',
      error: '',
    },
    plain: {
      false: 'items-center justify-center rounded-xl p-8 text-center',
      true: 'items-start gap-2 text-left',
    },
  },
  compoundVariants: [
    { plain: false, variant: 'empty', class: 'border border-dashed border-line-2' },
    { plain: false, variant: 'filtered', class: 'border border-dashed border-line-2' },
    // Solid rather than dashed: a dashed border reads as a placeholder for
    // something that belongs there, and a failure is not that.
    { plain: false, variant: 'error', class: 'border border-bad/40 bg-bad-soft/30' },
  ],
  defaultVariants: { variant: 'empty', plain: false },
})

/** The line's hexagon, at whatever size the caller asks for.
 *
 * Inlined rather than fetched: it is decorative, so it should not cost a
 * request, and it has to take the theme's colour - a file could not. */
export function EmptyMark({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 100 100" aria-hidden className={cn('size-14', className)}>
      <polygon
        points="50,5 89,27.5 89,72.5 50,95 11,72.5 11,27.5"
        fill="none"
        stroke="currentColor"
        strokeWidth={6}
        strokeLinejoin="round"
      />
    </svg>
  )
}

/** The same hexagon, cut by a slash. For a filter that matched nothing: the
 * shape is there, the contents are not. */
export function FilteredMark({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 100 100" aria-hidden className={cn('size-14', className)}>
      <polygon
        points="50,5 89,27.5 89,72.5 50,95 11,72.5 11,27.5"
        fill="none"
        stroke="currentColor"
        strokeWidth={6}
        strokeLinejoin="round"
      />
      <line x1="22" y1="78" x2="78" y2="22" stroke="currentColor" strokeWidth={6} strokeLinecap="round" />
    </svg>
  )
}

/** A hexagon with a corner broken out of it. For something that failed. */
export function ErrorMark({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 100 100" aria-hidden className={cn('size-14', className)}>
      <polyline
        points="50,5 89,27.5 89,72.5 50,95 11,72.5 11,27.5 50,5"
        fill="none"
        stroke="currentColor"
        strokeWidth={6}
        strokeLinejoin="round"
        strokeLinecap="round"
        // The gap is the break: the outline is drawn as a line rather than a
        // closed shape so one edge can be missing.
        strokeDasharray="150 34"
      />
    </svg>
  )
}

export interface EmptyStateProps
  extends Omit<HTMLAttributes<HTMLDivElement>, 'title'>,
    VariantProps<typeof emptyStateVariants> {
  /** What is not here, in the product's words. */
  title: ReactNode
  /** Why, or what to do about it. */
  body?: ReactNode
  /** The one thing worth doing here. An empty screen with no way out is a
   * dead end. */
  action?: ReactNode
  /** Something other than the default mark - a product's own illustration.
   * A plain empty state draws no mark, its own or the default. */
  mark?: ReactNode
}

export function EmptyState({
  variant = 'empty',
  plain = false,
  title,
  body,
  action,
  mark,
  className,
  ...props
}: EmptyStateProps) {
  const defaultMark =
    variant === 'error' ? (
      <ErrorMark className="text-bad/60" />
    ) : variant === 'filtered' ? (
      <FilteredMark className="text-line-2" />
    ) : (
      <EmptyMark className="text-line-2" />
    )

  if (plain) {
    return (
      <div className={cn(emptyStateVariants({ variant, plain }), className)} {...props}>
        <div>
          <p className={cn('text-sm', variant === 'error' ? 'text-bad' : 'text-dim')}>{title}</p>
          {body !== undefined && <p className="mt-0.5 text-xs text-faint">{body}</p>}
        </div>
        {action}
      </div>
    )
  }

  return (
    <div className={cn(emptyStateVariants({ variant, plain }), className)} {...props}>
      {mark ?? defaultMark}
      <div>
        <p className={cn('font-medium', variant === 'error' && 'text-bad')}>{title}</p>
        {body !== undefined && <p className="mt-1 text-sm text-dim">{body}</p>}
      </div>
      {action}
    </div>
  )
}
