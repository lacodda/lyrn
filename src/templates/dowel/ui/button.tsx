import { useId, type ButtonHTMLAttributes, type MouseEvent } from 'react'
import { cva, type VariantProps } from 'class-variance-authority'
import { useRender } from '@base-ui/react/use-render'
// `cn` comes from the package rather than being copied in beside the
// component (ADR 0002): a helper every primitive shares should update
// centrally, and a project installing a component already has the package for
// the theme. shadcn's own components import it from `@/lib/utils`; that is a
// per-project alias, and a copied file cannot know what it points at.
import { cn } from 'dowel-ui'

/*
 * Button.
 *
 * Six variants, because that is what the line's products actually reach for:
 * one primary action per screen, a quiet default, a soft accent for something
 * selected, a destructive one, an icon-only, and one that reads as a link.
 *
 * Every colour and every size is a token. There are no `dark:` utilities and
 * no raw values - the theme swaps underneath, so the same class list is
 * correct in both themes and in every product's accent.
 *
 * **Height is a control row, not a number.** `md` and `sm` stand on
 * `h-control` and `h-control-sm`, the rows every field of the set stands on,
 * so a button beside an input of the same size is the same height - and
 * `data-density` on a container reaches the button along with the field. They
 * used to say `h-9` and `h-7` literally, and a compact form came out with 32px
 * fields beside 36px buttons. `xs` is below the rows on purpose: it is for a
 * button inside something - a chat line, a chip, a table cell - and it grows
 * its hit area to the pointer floor rather than its box.
 *
 * **Every size says how big an icon is.** A text button used to size nothing,
 * so a lucide icon inside one drew at its own 24px - taller than the text,
 * across the whole line, and fixed at each call site by a `size-4` that the
 * next call site forgot. Each size now sizes an svg inside it, and only one
 * that has no size of its own: `[&_svg:not([class*=size-])]`. The guard is
 * not a nicety. The unguarded form the icon sizes used to have is a
 * descendant selector, one class and one element, and it outranks the single
 * class `size-5` written on the icon - so an explicit size at a call site was
 * silently overruled, and a `+` meant to be 10px drew at 14. The value is
 * unquoted (`size-` is an identifier) so the class sits in a plain string.
 *
 * **A disabled button can say why.** It used to be `pointer-events-none`, so a
 * `title` on it was dead: kilna's trash explained why a row could not be
 * restored in a tooltip nobody could ever raise. Disabled is now a look
 * (`data-disabled`) with the hover scoped away from it, and `disabledReason`
 * goes further: the button stays reachable by Tab and by the pointer, a press
 * does nothing, and the reason is its `title` and its accessible description.
 * A disabled control with no word for why is a wall; one that answers is an
 * instruction.
 */
export const buttonVariants = cva(
  [
    'inline-flex cursor-pointer items-center justify-center gap-1.5 whitespace-nowrap',
    'font-medium transition-colors',
    // Disabled is a state, not a colour: the button keeps its own hue and
    // fades, which reads the same whatever the accent is. Drawn from
    // `data-disabled`, which both kinds of disabled set, and the hovers below
    // are scoped away from it - so the pointer still reaches the button (and
    // its `title`) without the button lighting up to say "press me".
    'data-disabled:cursor-not-allowed data-disabled:opacity-50',
    '[&_svg]:shrink-0',
  ],
  {
    variants: {
      variant: {
        primary: 'rounded-md bg-accent font-semibold text-on-accent not-data-disabled:hover:bg-accent-2',
        ghost:
          'rounded-md border border-line text-dim not-data-disabled:hover:border-line-2 not-data-disabled:hover:text-text',
        soft: 'rounded-md bg-accent-soft text-accent not-data-disabled:hover:bg-accent-soft/60',
        danger: 'rounded-md text-bad not-data-disabled:hover:bg-bad-soft',
        icon: 'rounded-md text-dim not-data-disabled:hover:bg-soft not-data-disabled:hover:text-text',
        /*
         * A button that reads as a link: "Open", "Show all", "Undo" inside a
         * line of text or a widget's header. Not an `<a>` - it acts rather
         * than goes, and `render={<a href />}` is there for the one that
         * goes. It takes no size: it sits in text, so it takes the text's
         * size and height and has no box of its own, and its icon is an em
         * so it scales with whatever line it is in.
         */
        link: [
          'h-auto rounded-xs px-0 text-accent underline-offset-2 not-data-disabled:hover:underline',
          '[&_svg:not([class*=size-])]:size-[1em]',
        ],
      },
      size: {
        xs: 'target-min h-6 gap-1 px-2 text-xs [&_svg:not([class*=size-])]:size-3',
        sm: 'h-control-sm px-2.5 text-xs [&_svg:not([class*=size-])]:size-3.5',
        md: 'h-control px-3.5 text-sm [&_svg:not([class*=size-])]:size-4',
        'icon-xs': 'target-min size-5 [&_svg:not([class*=size-])]:size-3',
        'icon-sm': 'size-7 [&_svg:not([class*=size-])]:size-3.5',
        'icon-md': 'size-8 [&_svg:not([class*=size-])]:size-4',
      },
    },
    defaultVariants: { variant: 'ghost', size: 'md' },
  },
)

export interface ButtonProps
  extends ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  /**
   * Why the button cannot be pressed, in the product's words - read only
   * while `disabled`.
   *
   * With it, the button stays in the tab order and under the pointer
   * (`aria-disabled` rather than `disabled`), a press does nothing, and the
   * reason is both its `title` and its accessible description. Without it a
   * disabled button is a native disabled one: skipped by Tab, and silent.
   */
  disabledReason?: string
  /**
   * Render something else with the button's clothes on - a link, most often.
   *
   * Takes the element itself rather than a boolean: `render={<a href="…" />}`.
   * A function is also accepted, for the rare case that needs the props
   * before deciding what to build with them.
   */
  render?: useRender.RenderProp
}

export function Button({
  variant,
  size,
  render,
  className,
  type,
  disabled = false,
  disabledReason,
  onClick,
  ...props
}: ButtonProps) {
  const reasonId = useId()
  const explained = disabled && disabledReason !== undefined && disabledReason !== ''

  const button = useRender({
    render,
    defaultTagName: 'button',
    props: {
      // A `<button>` inside a form submits it unless told otherwise, which
      // surprises everyone once. When rendering as something else the
      // attribute is meaningless and would land on an `<a>`, so it is only
      // set for the element that has it - `render` is what says which.
      ...(render === undefined && type === undefined ? { type: 'button' } : { type }),
      // A link has no size: it takes the size of the text it sits in.
      className: cn(buttonVariants({ variant, size: variant === 'link' ? null : size }), className),
      'data-disabled': disabled ? '' : undefined,
      ...(explained
        ? {
            // Reachable, and inert. `aria-disabled` keeps the button in the tab
            // order so the reason can be heard; the click is swallowed here,
            // which also stops a submit button submitting its form.
            'aria-disabled': true,
            'aria-describedby': reasonId,
            title: disabledReason,
            onClick: (event: MouseEvent<HTMLButtonElement>) => event.preventDefault(),
          }
        : { disabled: disabled || undefined, onClick }),
      ...props,
    },
  })

  if (!explained) return button

  // The description lives beside the button rather than inside it: text
  // inside a button is part of its name, and "Restore - the work it belonged
  // to is gone" is not what the button is called. `hidden` takes it off the
  // screen and out of the flow; `aria-describedby` still reads it.
  return (
    <>
      {button}
      <span id={reasonId} hidden>
        {disabledReason}
      </span>
    </>
  )
}
