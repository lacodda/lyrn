import type { HTMLAttributes, ReactNode } from 'react'
import { cva, type VariantProps } from 'class-variance-authority'
import { cn } from 'dowel-ui'

/*
 * Alert.
 *
 * A message that stays on the screen, in the flow of the page, about the thing
 * next to it: this field could not be saved, this profile has no axes yet,
 * this export is out of date.
 *
 * Not a Toast, and the difference is worth stating because products keep
 * reaching for the wrong one. A toast is for something that just happened and
 * then goes away; an alert is for a condition that is still true and will
 * still be true after a reload. If dismissing it would be a lie, it is an
 * alert.
 *
 * Not a Banner either: a banner spans the application and speaks about the
 * whole of it. This speaks about what it sits beside.
 *
 * `role` is the caller's decision, and the default is deliberately quiet. An
 * alert that appears in response to something the reader just did should be
 * `role="alert"` so it is announced; one that is simply part of the page
 * should not be, or a screen reader interrupts itself reading the furniture.
 */

export const alertVariants = cva(
  ['flex gap-2.5 rounded-md border p-3 text-sm', '[&_svg]:mt-0.5 [&_svg:not([class*=size-])]:size-4 [&_svg]:shrink-0'],
  {
    variants: {
      tone: {
        neutral: 'border-line bg-soft text-dim',
        good: 'border-good/40 bg-good-soft text-good',
        warn: 'border-warn/40 bg-warn-soft text-warn',
        bad: 'border-bad/40 bg-bad-soft text-bad',
        info: 'border-info/40 bg-info-soft text-info',
      },
    },
    defaultVariants: { tone: 'neutral' },
  },
)

export interface AlertProps
  // `title` on a div is the browser's tooltip and is a string; here it is the
  // heading, and can be anything a product wants to draw.
  extends Omit<HTMLAttributes<HTMLDivElement>, 'title'>,
    VariantProps<typeof alertVariants> {
  /** Drawn before the text. The product's own, because an icon that means
   * "warning" here should be the one it means everywhere else in the product. */
  icon?: ReactNode
  /** The heading. Optional: a one-line alert does not need one. */
  title?: ReactNode
  /** Anything that goes at the end - a link to the thing that fixes it, a
   * dismiss button the product owns. */
  action?: ReactNode
}

export function Alert({ tone, icon, title, action, className, children, ...props }: AlertProps) {
  return (
    <div className={cn(alertVariants({ tone }), className)} {...props}>
      {icon}
      <div className="min-w-0 flex-1">
        {title !== undefined && <div className="font-semibold">{title}</div>}
        {children !== undefined && (
          <div className={cn('text-xs', title !== undefined && 'mt-0.5')}>{children}</div>
        )}
      </div>
      {action !== undefined && <div className="shrink-0">{action}</div>}
    </div>
  )
}
