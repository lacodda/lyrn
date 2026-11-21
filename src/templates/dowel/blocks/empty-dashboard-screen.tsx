import { useId, type ReactNode } from 'react'
import { cn } from 'dowel-ui'
import { Frame, Pane } from '@/components/ui/frame'
import { PageHeader } from '@/components/ui/page-header'
import { Progress } from '@/components/ui/progress'
import { StatRow, StatTile } from '@/components/ui/stat-tile'

/*
 * EmptyDashboardScreen.
 *
 * A dashboard on its first day: the few things to do before it has anything
 * to show, with the next one ready to press, and the figures it will show
 * once it does - drawn empty, so the screen says what it is for.
 *
 * A dashboard with nothing in it is the first screen a product shows anyone,
 * and the line's products drew it as a dashboard with every number zero - a
 * row of tiles saying 0, 0, 0 and a chart with no line, which reads as broken
 * rather than as new. A zero is a claim: nothing happened. On the first day
 * nothing has had the chance to.
 *
 * So the screen leads with what to do. The steps are a list in order, each
 * marked done or not in words as well as a mark, and only the first one not
 * yet done carries its action: one next thing, not four buttons competing.
 * The figures follow, under their own heading, with a dash where a number
 * will be - the shape of the dashboard to come, not a report of nothing.
 */

export interface FirstStep {
  id: string
  title: ReactNode
  /** What doing it gets you, in a line. */
  description?: ReactNode
  done: boolean
  /** What does it - drawn on the first step not yet done, so there is one
   * next thing to press. */
  action?: ReactNode
}

/** A figure the dashboard will show, drawn here without a value. */
export interface FutureFigure {
  label: ReactNode
}

/** Every word the screen shows, in the product's language. */
export interface EmptyDashboardScreenWords {
  /** The list's heading: "Getting started". */
  steps: string
  /** How far along: "2 of 4 done". */
  progress: (done: number, total: number) => string
  /** Said beside a step's mark, for a reader who cannot see it: "Done", "Not yet". */
  done: string
  notYet: string
  /** The figures' heading: "What will show here". */
  figures: string
}

export interface EmptyDashboardScreenProps {
  /** "Welcome to kilna". */
  title: ReactNode
  /** A line under the title that says what the dashboard is for. */
  description?: ReactNode
  /** Buttons at the end of the header - the product's own, such as "Import data". */
  actions?: ReactNode
  /** The first steps, in order. Each is marked done or not, and only the first one not yet done carries its `action`. */
  steps: readonly FirstStep[]
  /** The figures the full dashboard will have. Leave it out to show only the steps. */
  figures?: readonly FutureFigure[]
  /** Every word the screen shows, in the product's language. The block has no default text. */
  words: EmptyDashboardScreenWords
  className?: string
}

function CheckIcon() {
  return (
    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden className="size-3.5">
      <path d="M3.5 8.5l3 3 6-7" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  )
}

export function EmptyDashboardScreen({
  title,
  description,
  actions,
  steps,
  figures,
  words,
  className,
}: EmptyDashboardScreenProps) {
  const figuresId = useId()
  const done = steps.filter((step) => step.done).length
  const next = steps.findIndex((step) => !step.done)

  return (
    <Frame
      head={<PageHeader title={title} description={description} actions={actions} className="mb-0 w-full" />}
      className={className}
    >
      <div className="flex min-h-0 w-full max-w-3xl flex-1 flex-col gap-2.5">
        {/* Sized by its steps and shrinking when the window is shorter: then
            the steps scroll and the figures stay in sight under them. */}
        <Pane
          label={words.steps}
          className="flex-initial"
          head={
            <div className="flex w-full flex-wrap items-center gap-x-4 gap-y-2">
              <h2 className="text-base font-semibold text-text">{words.steps}</h2>
              {/* The count in words beside a bare bar: the bar's own label
                  row would add a percentage, and "25%" of four steps says
                  less than "1 of 4". */}
              <div className="ml-auto flex items-center gap-3">
                <span className="text-xs text-dim tabular-nums">{words.progress(done, steps.length)}</span>
                <Progress value={done} max={Math.max(steps.length, 1)} label={words.steps} className="w-32" />
              </div>
            </div>
          }
          bodyClassName="p-1.5"
        >
          <ol className="m-0 flex list-none flex-col p-0">
            {steps.map((step, index) => {
              const current = index === next
              return (
                <li
                  key={step.id}
                  aria-current={current ? 'step' : undefined}
                  className={cn(
                    'flex items-start gap-3 rounded-md px-2.5 py-2.5',
                    current && 'bg-accent-soft',
                  )}
                >
                  <span
                    className={cn(
                      'mt-0.5 flex size-5 shrink-0 items-center justify-center rounded-full text-2xs font-semibold tabular-nums',
                      step.done
                        ? 'bg-good-soft text-good'
                        : current
                          ? 'bg-accent text-on-accent'
                          : 'border border-line-2 text-dim',
                    )}
                  >
                    {step.done ? <CheckIcon /> : index + 1}
                  </span>
                  <div className="flex min-w-0 flex-1 flex-col gap-0.5">
                    <span className={cn('text-sm font-medium', step.done ? 'text-dim line-through' : 'text-text')}>
                      {step.title}
                      {/* The mark says it to the eye; this says it to a
                          reader who cannot see a tick or a strike. */}
                      <span className="sr-only">, {step.done ? words.done : words.notYet}</span>
                    </span>
                    {step.description === undefined ? null : (
                      <span className="text-xs text-dim">{step.description}</span>
                    )}
                  </div>
                  {current && step.action !== undefined ? (
                    <div className="flex shrink-0 items-center">{step.action}</div>
                  ) : null}
                </li>
              )
            })}
          </ol>
        </Pane>

        {figures === undefined || figures.length === 0 ? null : (
          <section
            aria-labelledby={figuresId}
            className="flex shrink-0 flex-col gap-3 rounded-lg border border-dashed border-line-2 p-4"
          >
            <h2 id={figuresId} className="caption">
              {words.figures}
            </h2>
            {/* A dash, not a zero: nothing has had the chance to happen. */}
            <StatRow>
              {figures.map((figure, index) => (
                <StatTile key={index} label={figure.label} value={<span className="text-faint">—</span>} />
              ))}
            </StatRow>
          </section>
        )}
      </div>
    </Frame>
  )
}
