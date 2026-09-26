import type { HTMLAttributes } from 'react'
import { cva, type VariantProps } from 'class-variance-authority'
import { cn } from 'dowel-ui'

/*
 * Panel.
 *
 * The raised surface everything else sits on. It is the one place a screen
 * gets its structure from, so it stays deliberately plain: a ground, a
 * hairline, a corner.
 *
 * The two products it came from disagreed about that corner - one used 16px,
 * the other 12px, for the component with the same name and the same purpose.
 * It is `lg` here, and `SectionLabel` is included because a panel almost
 * always has one and every product wrote its own.
 */
export const panelVariants = cva('bg-raise', {
  variants: {
    variant: {
      /* The default: a surface that sits on the page. */
      raised: 'rounded-lg border border-line',
      /* For something that has left the page - a menu, a popover. The shadow
       * is what says how far away it is. */
      floating: 'rounded-lg border border-line shadow-raise',
      /* Inside another panel, where a second border would be a box in a box. */
      inset: 'rounded-inner bg-soft',
    },
  },
  defaultVariants: { variant: 'raised' },
})

export interface PanelProps
  extends HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof panelVariants> {}

export function Panel({ variant, className, ...props }: PanelProps) {
  return <div className={cn(panelVariants({ variant }), className)} {...props} />
}

/**
 * The small uppercase caption above a block of content.
 *
 * Its tracking is the one the products argued about - 0.08em in six files and
 * 0.09em in three - and it is a token now, so the argument cannot recur.
 */
export function SectionLabel({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        'caption flex items-center gap-2',
        className,
      )}
      {...props}
    />
  )
}
