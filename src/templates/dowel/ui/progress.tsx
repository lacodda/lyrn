import { Progress as Base } from '@base-ui/react/progress'
import { cva, type VariantProps } from 'class-variance-authority'
import { cn } from 'dowel-ui'

/*
 * How far along something is.
 *
 * The distinction the component is built on, and the one products collapse:
 *
 *   **determinate** - the fraction is known. The bar fills to it, and a reader
 *   can tell how long is left.
 *
 *   **indeterminate** - something is happening and nobody knows how much is
 *   left. The bar says exactly that, by moving without filling.
 *
 * Collapsing them means picking a number that is not true - a bar that creeps
 * to 90% and waits there is the commonest version, and it is a lie the reader
 * learns to distrust, after which no progress bar in the product means
 * anything. `value={undefined}` is the honest answer, and it is the default.
 *
 * Base UI carries the role, the announcement and the value clamping. What is
 * here is the clothes, and the rule about which of the two states is drawn.
 *
 * Not a Spinner. A spinner says "working" in a corner; this says "working, and
 * here is the shape of it" across a width. Where the shape of what is coming
 * is known, a Skeleton says more than either.
 */

export const progressVariants = cva('w-full overflow-hidden rounded-full bg-soft', {
  variants: {
    size: {
      sm: 'h-1',
      md: 'h-2',
    },
    tone: {
      accent: '',
      good: '',
      warn: '',
      bad: '',
    },
  },
  defaultVariants: { size: 'md', tone: 'accent' },
})

const fillVariants = cva('h-full rounded-full transition-[width] duration-base', {
  variants: {
    tone: {
      accent: 'bg-accent',
      good: 'bg-good',
      warn: 'bg-warn',
      bad: 'bg-bad',
    },
  },
  defaultVariants: { tone: 'accent' },
})

/* The stripes take their colour from `currentColor`, so the tone arrives as a
 * text colour rather than a background - the same four names, said the other
 * way round. */
const stripeVariants = cva('h-full w-full rounded-full opacity-40', {
  variants: {
    tone: {
      accent: 'text-accent',
      good: 'text-good',
      warn: 'text-warn',
      bad: 'text-bad',
    },
  },
  defaultVariants: { tone: 'accent' },
})

export interface ProgressProps
  extends Omit<Base.Root.Props, 'className' | 'value'>,
    VariantProps<typeof progressVariants> {
  /** The fraction done, 0 to `max`. Leave it out when it is not known - that
   * is not a missing value but a different, honest state. */
  value?: number | null
  max?: number
  /** What is progressing, for a screen reader. Required: a bar with no name is
   * announced as a percentage of nothing, and the word belongs to the product. */
  label: string
  className?: string
  /** A visible label beside the bar, when there is room. */
  children?: React.ReactNode
}

export function Progress({
  value,
  max = 100,
  size,
  tone,
  label,
  className,
  children,
  ...props
}: ProgressProps) {
  const indeterminate = value === undefined || value === null

  return (
    <Base.Root
      value={indeterminate ? null : value}
      max={max}
      aria-label={label}
      className={cn('flex w-full flex-col gap-1.5', className)}
      {...props}
    >
      {children !== undefined && (
        <div className="flex items-baseline justify-between text-xs text-dim">
          {children}
          {/* The number only where there is one. Showing "0%" for an unknown
            * amount is the same lie as a bar that creeps to 90%. */}
          {!indeterminate && (
            <span className="tabular-nums">{Math.round((value / max) * 100)}%</span>
          )}
        </div>
      )}

      <Base.Track className={cn(progressVariants({ size, tone }))}>
        {indeterminate ? (
          /* Stripes across the whole track, not a filled bar.
           *
           * A full-width fill was the first version and it was wrong in the
           * one way that matters: measured on the stand, it drew 384px of a
           * 384px track - a reader glancing at it sees "done", which is the
           * opposite of what the state means. Pulsing did not rescue it, and
           * under `prefers-reduced-motion` the pulse stops and a solid,
           * complete-looking bar is all that remains.
           *
           * Stripes cannot be read as a fraction at all: there is no edge to
           * take for a boundary. They are drawn with a gradient rather than a
           * `@keyframes` of their own, because a name in the theme is a
           * contract the line carries forever - and this needs no animation to
           * say what it says. */
          <div
            className={cn(stripeVariants({ tone }))}
            style={{
              backgroundImage:
                'repeating-linear-gradient(45deg, currentColor 0 6px, transparent 6px 12px)',
            }}
          />
        ) : (
          <Base.Indicator className={cn(fillVariants({ tone }))} />
        )}
      </Base.Track>
    </Base.Root>
  )
}
