import type { CSSProperties, ReactNode, Ref } from 'react'
import { ScrollArea as Base } from '@base-ui/react/scroll-area'
import { cva } from 'class-variance-authority'
import { cn } from 'dowel-ui'

/*
 * A box that scrolls, with the line's scrollbar drawn over its content.
 *
 * The line has one rule for scrollbars: a thin bar laid over the content,
 * never a gutter that takes width. A gutter is a column of the layout that
 * comes and goes with the length of the content, so a list that grows past
 * the fold pushes everything beside it sideways by the width of a bar - and
 * on a platform that draws a classic bar, the product wears someone else's
 * chrome in the middle of its own screen. The theme's global scrollbar rules
 * make native bars thin, but thin is still a gutter wherever the platform
 * draws one; only a bar that is not the browser's can promise to take no room.
 *
 * So the viewport hides its native bars and Base UI draws the thumb, sized and
 * placed from the scroll position. The bar is invisible at rest and appears
 * while the pointer is over the area or while it scrolls, because a bar that
 * is always there is a line across the content that nobody asked for. The
 * track is wider than the thumb and the thumb thickens under the pointer: a
 * four-pixel thumb reads as a hint and is hopeless as a grip, and the grip is
 * what somebody reaching for it wants. Both axes are handled, and Base UI
 * mounts a bar only for an axis that actually overflows.
 *
 * The keyboard is the part a hand-drawn scrollbar usually loses. A box that
 * scrolls must be reachable, or a reader without a pointer cannot see what is
 * below the fold (axe's `scrollable-region-focusable`), so Base UI makes the
 * viewport a tab stop exactly when it overflows. A tab stop has to be called
 * something, and the viewport is then a named group - which is why `label` is
 * required: whether the content will overflow is not known when the product
 * is written, and a name added only for the long case is a name forgotten for
 * it.
 *
 * `fade` softens the edges where there is more to see. It reads Base UI's
 * overflow distances, so an edge that is already at its end is left sharp -
 * a fade on both ends of a list scrolled to the top would say there is
 * something above when there is not.
 *
 * `orientation` says which way the content may move. Base UI sizes its content
 * box to `fit-content`, which is right for a table that scrolls sideways and
 * wrong for a list: a row that should truncate to the column instead widens
 * the box, and the list scrolls sideways by the length of its longest title.
 * `vertical` holds the content to the viewport's width; `horizontal` holds it
 * to the viewport's height.
 */

type Orientation = NonNullable<ScrollAreaProps['orientation']>

/* Inline, because the style Base UI writes is inline too - `min-width:
 * fit-content` on the content and `overflow: scroll` on the viewport - and a
 * class loses to it. */
const contentStyle: Record<Orientation, CSSProperties | undefined> = {
  both: undefined,
  vertical: { minWidth: 0 },
  horizontal: undefined,
}
const viewportStyle: Record<Orientation, CSSProperties | undefined> = {
  both: undefined,
  vertical: { overflowX: 'hidden' },
  horizontal: { overflowY: 'hidden' },
}

export const scrollAreaViewportVariants = cva(
  cn(
    'size-full rounded-[inherit] outline-none',
    'focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent',
    // On paper nothing scrolls: what was below the fold is printed below it.
    // Important, because Base UI writes the overflow inline.
    'print:h-auto print:overflow-visible!',
  ),
  {
    variants: {
      /* A mask, not an overlay gradient: a gradient would have to be painted
       * in the colour of whatever is behind the area, which the component
       * cannot know. A mask fades the content itself, over any ground. Each
       * edge fades by at most `--fade`, and by less while the content is
       * nearer its end than that - so the fade arrives as scrolling starts
       * rather than switching on. */
      fade: {
        true: cn(
          '[--fade:calc(var(--spacing)*6)] [mask-composite:intersect]',
          '[mask-image:linear-gradient(to_bottom,transparent,var(--color-black)_min(var(--fade),var(--scroll-area-overflow-y-start,0px)),var(--color-black)_calc(100%_-_min(var(--fade),var(--scroll-area-overflow-y-end,0px))),transparent),linear-gradient(to_right,transparent,var(--color-black)_min(var(--fade),var(--scroll-area-overflow-x-start,0px)),var(--color-black)_calc(100%_-_min(var(--fade),var(--scroll-area-overflow-x-end,0px))),transparent)]',
        ),
        false: '',
      },
    },
    defaultVariants: { fade: false },
  },
)

/* The track is the grab zone and the thumb sits at its outer edge. Hidden at
 * rest; shown while the pointer is over the area or it scrolls, and held
 * while the thumb is being dragged, since the pointer may leave the area
 * mid-drag.
 *
 * On the sticky layer, and after the content: a table that pins a column
 * (`sticky right-0 z-2`) or its heading (`--z-sticky`) used to cover the
 * thumb, which was then neither seen nor taken, and a press at the edge
 * opened the row under it - kilna's catalogue, measured with
 * `elementFromPoint`. Equal to the heading's layer, the bar still wins it:
 * it comes later in the tree. */
const scrollbar = cn(
  'group/bar flex p-0.5 opacity-0 print:hidden [transition:opacity_var(--duration-slow)_var(--ease-out)] [z-index:var(--z-sticky)]',
  'data-[hovering]:opacity-100 data-[scrolling]:opacity-100 active:opacity-100',
  'data-[orientation=vertical]:w-2.5 data-[orientation=vertical]:justify-end',
  'data-[orientation=horizontal]:h-2.5 data-[orientation=horizontal]:flex-col data-[orientation=horizontal]:justify-end',
)

const thumb = cn(
  'rounded-full bg-line-2 group-hover/bar:bg-dim',
  '[transition:width_var(--duration-quick)_var(--ease-out),height_var(--duration-quick)_var(--ease-out),background-color_var(--duration-quick)_var(--ease-out)]',
  'data-[orientation=vertical]:w-1 group-hover/bar:data-[orientation=vertical]:w-full',
  'data-[orientation=horizontal]:h-1 group-hover/bar:data-[orientation=horizontal]:h-full',
)

export interface ScrollAreaProps {
  /** Names the viewport, which becomes a tab stop when its content overflows:
   * "Recent activity", "Diff of main.rs". No default - it is the product's
   * word. */
  label: string
  /** Fade the edges that have more content past them. */
  fade?: boolean
  /** Which way the content may move. `vertical` for a list whose rows
   * truncate to the column; `both` for a table or a picture. */
  orientation?: 'both' | 'vertical' | 'horizontal'
  /** The scrolling element itself, for a product that sets or reads its
   * position - a virtual list, a scroll-to-latest. */
  viewportRef?: Ref<HTMLDivElement>
  /** On the root: its size is the area's size, so the height goes here. */
  className?: string
  /** On the scrolling element, for padding that should scroll with the content. */
  viewportClassName?: string
  children: ReactNode
}

export function ScrollArea({
  label,
  fade = false,
  orientation = 'both',
  viewportRef,
  className,
  viewportClassName,
  children,
}: ScrollAreaProps) {
  return (
    <Base.Root className={cn('relative min-h-0 min-w-0 overflow-hidden print:overflow-visible', className)}>
      <Base.Viewport
        ref={viewportRef}
        // Base UI gives the viewport `presentation`, which is right for a box
        // nobody can focus and wrong for one that is a tab stop: the role is
        // dropped the moment it is focusable, and the name goes with it.
        role="group"
        aria-label={label}
        className={cn(scrollAreaViewportVariants({ fade }), viewportClassName)}
        style={viewportStyle[orientation]}
      >
        {/* The content box is what the horizontal overflow is measured on:
          * without it a wide child is squeezed to the viewport's width and
          * nothing ever overflows sideways. */}
        <Base.Content style={contentStyle[orientation]}>{children}</Base.Content>
      </Base.Viewport>
      {orientation === 'horizontal' ? null : (
        <Base.Scrollbar orientation="vertical" className={scrollbar}>
          <Base.Thumb className={thumb} />
        </Base.Scrollbar>
      )}
      {orientation === 'vertical' ? null : (
        <Base.Scrollbar orientation="horizontal" className={scrollbar}>
          <Base.Thumb className={thumb} />
        </Base.Scrollbar>
      )}
    </Base.Root>
  )
}
