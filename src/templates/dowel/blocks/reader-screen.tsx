import { useEffect, useEffectEvent, useId, useRef, useState, type PointerEvent, type ReactNode } from 'react'
import { cn } from 'dowel-ui'
import { Progress } from '@/components/ui/progress'

/*
 * ReaderScreen.
 *
 * A text read from top to bottom, a paragraph at a time: a story, a chapter,
 * a lesson's reading. Harvested from rhapsod, whose reader it was, and grown
 * by hilvan, which reads with a hand on every word - the line decided on
 * 11.09 that the two readers are one block.
 *
 * What is the same in both, and so is the block's:
 *
 * - **The measure.** A column a line of prose fits in, the text a step larger
 *   than the screen's own on a phone, and the leading a long read needs.
 * - **Where the reader is.** The paragraph being read is the last one whose
 *   top has passed the middle of the window - the line the eye is on, not the
 *   one about to appear - followed as the page scrolls and told to the
 *   product whenever it changes, so the product can keep the place.
 * - **Back where they stopped.** The paragraph the product names in `start`
 *   is scrolled to once, with room above it: a paragraph flush against the
 *   top edge reads as a page that was cut.
 * - **A sweep across a paragraph.** A finger drawn to the right across one
 *   tells the product (`onSwipe`) - hilvan's "I know the rest of it". Only a
 *   finger, only rightwards, and dropped the moment it runs down the page: a
 *   reader scrolling must never set off anything, however far the scroll
 *   drifts sideways.
 * - **A bar that stands at the foot.** How far through the text the reader is,
 *   and whatever the product keeps in reach there.
 *
 * What is the product's: every word. A paragraph is drawn by `renderParagraph`
 * - plain text, rich runs, a button per word - and the block never looks
 * inside it.
 */

/** How far a finger has to travel to the right for a sweep. */
export const SWIPE = 80
/** Beyond this much vertical movement, or this much back to the left, it is
 * not a sweep. */
export const SLOPE = 30

export type SwipeVerdict = 'swept' | 'abandon' | 'watching'

/** What a finger's movement to `x`, `y` means for a sweep that began at
 * `start`. Vertical first: a drag that runs down the page is a scroll. */
export function swipeVerdict(start: { x: number; y: number }, x: number, y: number): SwipeVerdict {
  if (Math.abs(y - start.y) > SLOPE) return 'abandon'
  if (start.x - x > SLOPE) return 'abandon'
  if (x - start.x >= SWIPE) return 'swept'
  return 'watching'
}

/** The paragraph being read: the last one whose top has passed `middle`.
 * `null` for a paragraph not on the page. The first, when none has. */
export function paragraphAt(tops: readonly (number | null)[], middle: number): number {
  let at = 0
  tops.forEach((top, index) => {
    if (top !== null && top < middle) at = index
  })
  return at
}

/** What the text scrolls in: the nearest box around it that scrolls, or the
 * window. A reader in a screen whose column holds still scrolls in a pane,
 * and following the window there would follow nothing. */
function scrollerOf(element: HTMLElement | null): HTMLElement | null {
  for (let node = element?.parentElement ?? null; node !== null; node = node.parentElement) {
    if (/(auto|scroll|overlay)/.test(getComputedStyle(node).overflowY)) return node
  }
  return null
}

/** Every word the screen shows, in the product's language. */
export interface ReaderScreenWords {
  /** The progress bar's name, given where the reader is: "Paragraph 3 of 12". */
  progress: (paragraph: number, count: number) => string
}

export interface ReaderScreenProps {
  /** The text's title. */
  title: ReactNode
  /** Under the title: an author, a chapter, how long it reads. */
  byline?: ReactNode
  /** Above the text: a line on how to read it, the chapters' link. */
  lead?: ReactNode
  /** How many paragraphs the text has. */
  count: number
  /** A paragraph, words and all. */
  renderParagraph: (index: number) => ReactNode
  /** Where the reader stopped, scrolled to once when the screen opens. `0`
   * or nothing: the top. */
  start?: number | null
  /** Told the paragraph being read whenever it changes. */
  onParagraphChange?: (index: number) => void
  /** Told a paragraph a finger swept to the right. Without it, nothing
   * listens for a sweep. */
  onSwipe?: (index: number) => void
  /** A paragraph drawn marked - the one just swept, the one being heard. */
  marked?: number | null
  /** What stands in the foot bar beside the progress: an action in reach. */
  foot?: ReactNode
  /** What follows the text: the next chapter, "I have read it". */
  after?: ReactNode
  /** Every word the screen says itself, in the product's language. */
  words: ReaderScreenWords
  className?: string
}

export function ReaderScreen({
  title,
  byline,
  lead,
  count,
  renderParagraph,
  start,
  onParagraphChange,
  onSwipe,
  marked,
  foot,
  after,
  words,
  className,
}: ReaderScreenProps) {
  const titleId = useId()
  const root = useRef<HTMLDivElement>(null)
  const paragraphs = useRef<(HTMLParagraphElement | null)[]>([])
  const at = useRef(-1)
  // The paragraph the bar shows as reached; where the reader stopped until
  // the page has been followed.
  const [shown, setShown] = useState(Math.max(0, start ?? 0))

  // Back where the reader stopped, once: a place that moves under a reader
  // who has started scrolling is worse than none.
  const restored = useRef(false)
  useEffect(() => {
    if (restored.current) return
    restored.current = true
    // Nothing to do for the top: a paragraph not past it is where the page is.
    const place = start ?? 0
    if (place > 0) paragraphs.current[place]?.scrollIntoView({ block: 'start' })
  }, [start])

  // Follow the reader down the page, a frame at a time however fast the
  // scroll events come.
  const follow = useEffectEvent((scroller: HTMLElement | null) => {
    // A paragraph that has left the page leaves its ref empty.
    const tops = paragraphs.current.map((element) => element?.getBoundingClientRect().top ?? null)
    const box = scroller?.getBoundingClientRect()
    const middle = box === undefined ? window.innerHeight / 2 : box.top + box.height / 2
    const next = paragraphAt(tops, middle)
    if (next === at.current) return
    at.current = next
    setShown(next)
    onParagraphChange?.(next)
  })
  useEffect(() => {
    const scroller = scrollerOf(root.current)
    const target: HTMLElement | Window = scroller ?? window
    let frame = 0
    const onScroll = () => {
      if (frame) return
      frame = window.requestAnimationFrame(() => {
        frame = 0
        follow(scroller)
      })
    }
    target.addEventListener('scroll', onScroll, { passive: true })
    return () => {
      target.removeEventListener('scroll', onScroll)
      if (frame) window.cancelAnimationFrame(frame)
    }
  }, [])

  // A sweep, watched from a finger's first touch on a paragraph.
  const sweep = useRef<{ x: number; y: number; paragraph: number } | null>(null)
  const down = (paragraph: number) => (event: PointerEvent) => {
    sweep.current = event.pointerType === 'touch' ? { x: event.clientX, y: event.clientY, paragraph } : null
  }
  const move = (event: PointerEvent) => {
    const from = sweep.current
    if (from === null) return
    const verdict = swipeVerdict(from, event.clientX, event.clientY)
    if (verdict === 'watching') return
    sweep.current = null
    if (verdict === 'swept') onSwipe?.(from.paragraph)
  }
  const end = () => {
    sweep.current = null
  }

  return (
    <div ref={root} className={cn('mx-auto flex w-full max-w-prose flex-col gap-8', className)}>
      <article aria-labelledby={titleId} className="flex flex-col gap-6">
        <header className="flex flex-col gap-1">
          <h1 id={titleId} className="text-2xl font-semibold tracking-tight text-balance text-text">
            {title}
          </h1>
          {byline !== undefined && <p className="text-base text-dim">{byline}</p>}
          {lead !== undefined && <div className="mt-2 text-base text-dim">{lead}</div>}
        </header>
        <div className="flex flex-col gap-5">
          {Array.from({ length: count }, (_, index) => (
            <p
              key={index}
              data-paragraph={index}
              ref={(element) => {
                paragraphs.current[index] = element
              }}
              onPointerDown={onSwipe && down(index)}
              onPointerMove={onSwipe && move}
              onPointerUp={onSwipe && end}
              onPointerCancel={onSwipe && end}
              className={cn(
                // A step larger on a phone, held a little further from the
                // eye, and the leading a long read needs.
                'text-xl leading-[1.75] text-pretty text-text sm:text-lg sm:leading-[1.8]',
                // Room above a paragraph scrolled to, so it does not read as
                // a page cut at its first line.
                'scroll-mt-18 rounded-sm transition-colors',
                // Sideways is the sweep's; up and down stay the page's.
                onSwipe && 'touch-pan-y',
                marked === index && 'bg-accent-soft',
              )}
            >
              {renderParagraph(index)}
            </p>
          ))}
        </div>
      </article>

      {after !== undefined && <div className="flex flex-col gap-3 border-t border-line pt-6">{after}</div>}

      <div className="sticky bottom-0 flex items-center gap-3 border-t border-line bg-bg py-3 [z-index:var(--z-sticky)]">
        <Progress
          value={count === 0 ? 0 : shown + 1}
          max={Math.max(count, 1)}
          size="sm"
          label={words.progress(shown + 1, count)}
          className="min-w-0 flex-1"
        />
        {foot}
      </div>
    </div>
  )
}
