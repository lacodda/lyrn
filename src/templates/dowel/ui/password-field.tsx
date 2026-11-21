import { useId, useState, type Ref } from 'react'
import { cn } from 'dowel-ui'
import { fieldClasses } from './input'

/*
 * PasswordField - a password, and the button that shows it.
 *
 * The reveal is the whole component, and it is not a convenience. A masked
 * field is the only one in a form where a typo cannot be seen, so people
 * either paste (fine) or type slowly and get it wrong anyway; the toggle is
 * what turns an unverifiable field into a checkable one, and it is why long
 * passphrases became usable at all.
 *
 * What it costs is a moment where the password is on the screen, so the
 * component states its two rules rather than leaving them to each product:
 *
 *   - it always starts masked, and there is no prop to start it revealed;
 *   - revealing is the reader's own action, never a default and never
 *     something a form can turn on for them.
 *
 * The button is a real button with a real name, and the name changes with the
 * state - "Show password" / "Hide password". That is what a screen reader
 * announces, and it is the one place the component needs words, so they are
 * required props. A default here would ship English inside a primitive.
 *
 * `autoComplete` is not defaulted either. The right value is the product's
 * to know: `current-password` on a login, `new-password` on a sign-up, and
 * getting it wrong is how a password manager fills the wrong box.
 */

export interface PasswordFieldProps {
  /** The controlled password. */
  value?: string
  /** The starting password of an uncontrolled field. */
  defaultValue?: string
  /** Called with the new text on every change. */
  onValueChange?: (value: string) => void
  /** What the reveal button is called while the password is hidden. */
  showLabel: string
  /** ...and while it is showing. */
  hideLabel: string
  /** `current-password` for a login, `new-password` for a sign-up. */
  autoComplete?: string
  /** Shown in the box while it is empty. */
  placeholder?: string
  /** Turns the field and its reveal button off. */
  disabled?: boolean
  /** The password can be read and selected but not changed. */
  readOnly?: boolean
  /** Marks the field as required, for a form to check. */
  required?: boolean
  /** The name the password is submitted under in a form. */
  name?: string
  /** The `id` of the input, which the reveal button controls. A generated one
   * if omitted. */
  id?: string
  /** A ref to the `<input>`, to focus it after a failed sign-in. */
  ref?: Ref<HTMLInputElement>
  /** Names the field for a screen reader, when there is no visible label. */
  'aria-label'?: string
  /** The `id` of the element that describes the field - the rules a new
   * password must meet. */
  'aria-describedby'?: string
  className?: string
}

export function PasswordField({
  value,
  defaultValue,
  onValueChange,
  showLabel,
  hideLabel,
  className,
  disabled,
  ref,
  ...props
}: PasswordFieldProps) {
  /* Always false to begin with. Deliberately local state with no prop to set
   * it: a password that arrives on screen without the reader asking is the
   * one failure this component must not have. */
  const [revealed, setRevealed] = useState(false)
  const inputId = useId()

  return (
    <div
      className={cn(
        fieldClasses,
        'flex h-control items-stretch overflow-hidden p-0',
        'focus-within:outline-2 focus-within:outline-offset-0 focus-within:outline-accent',
        className,
      )}
    >
      <input
        {...props}
        ref={ref}
        id={props.id ?? inputId}
        type={revealed ? 'text' : 'password'}
        value={value}
        defaultValue={defaultValue}
        disabled={disabled}
        onChange={(event) => onValueChange?.(event.target.value)}
        className={cn(
          // 16px under a finger, for the reason `fieldClasses` gives.
          'w-full min-w-0 bg-transparent px-2.5 text-sm pointer-coarse:text-lg text-text placeholder:text-faint',
          'outline-none disabled:cursor-not-allowed',
        )}
      />

      <button
        type="button"
        // Not a submit button, and not in the tab order ahead of the field it
        // belongs to - it sits after the input, which is where Tab reaches it.
        onClick={() => setRevealed((was) => !was)}
        disabled={disabled}
        aria-label={revealed ? hideLabel : showLabel}
        aria-pressed={revealed}
        aria-controls={props.id ?? inputId}
        className={cn(
          'flex w-9 shrink-0 items-center justify-center text-dim',
          'transition-colors hover:bg-soft hover:text-text',
          'focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent',
          'disabled:pointer-events-none disabled:opacity-50',
        )}
      >
        {revealed ? (
          /* An eye with a stroke through it: hiding is the action offered
           * while the password is visible. */
          <svg viewBox="0 0 20 20" className="size-4" fill="none" aria-hidden>
            <path
              d="M4 4l12 12M8.5 8.7a2 2 0 002.8 2.8M6.3 6.4C4.4 7.5 3 9.2 2.5 10c1.2 2.2 4 5 7.5 5 1.3 0 2.5-.4 3.5-1M9 5.1c.3 0 .7-.1 1-.1 3.5 0 6.3 2.8 7.5 5-.3.5-.8 1.3-1.6 2.1"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
            />
          </svg>
        ) : (
          <svg viewBox="0 0 20 20" className="size-4" fill="none" aria-hidden>
            <path
              d="M2.5 10C3.7 7.8 6.5 5 10 5s6.3 2.8 7.5 5c-1.2 2.2-4 5-7.5 5s-6.3-2.8-7.5-5z"
              stroke="currentColor"
              strokeWidth="1.5"
            />
            <circle cx="10" cy="10" r="2.2" stroke="currentColor" strokeWidth="1.5" />
          </svg>
        )}
      </button>
    </div>
  )
}
