import { useId, type ReactElement, type ReactNode } from 'react'
import { Field as Base } from '@base-ui/react/field'
import { Fieldset } from '@base-ui/react/fieldset'
import { cn } from 'dowel-ui'

/*
 * Field - the label, the control, and what the form has to say about it.
 *
 * Every form is the same four parts repeated: a name for the control, the
 * control, sometimes a hint, and sometimes an error. Written by hand each
 * time, they drift - the label loses its `htmlFor`, the hint is a `<div>` no
 * screen reader mentions, the error appears in red and is announced by
 * nothing at all. This is that arrangement, once.
 *
 * The wiring is the point, and Base UI does it: the label points at the
 * control, the description and the error are named by `aria-describedby`, and
 * the control is marked invalid while an error is showing. None of that is
 * visible when it works, which is exactly why a product stops doing it.
 *
 * `error` is a string this is given, not a rule this enforces. dowel has no
 * opinion about where the string came from - Base UI's own `validate`, a
 * schema, react-hook-form, or a server that just said no - because a design
 * system that picked a form library would be choosing for products that
 * already chose. The one thing it insists on is that an error, once it
 * exists, is announced and points at the field it belongs to.
 *
 * The label is always rendered. A field whose name is only a placeholder
 * loses that name the moment someone types, and a placeholder is not a label
 * to anything that reads the page aloud; `labelHidden` takes it off the
 * screen and leaves it in the accessibility tree.
 */

export interface FieldProps {
  /** What the control is called. Always rendered; `labelHidden` only takes it
   * off the screen. */
  label: ReactNode
  /** The control itself - Input, Textarea, Select, Checkbox, anything.
   *
   * A single element rather than arbitrary nodes, because it is handed to
   * Base UI to carry the field's id and its `aria-*` wiring. Two children, or
   * a bare string, would leave the label naming nothing. */
  children: ReactElement
  /** A hint under the control. Hidden while an error is showing: two lines of
   * small print under one field is one line too many, and the error is the
   * one that matters. */
  help?: ReactNode
  /** What is wrong, if anything. Its presence is what marks the control
   * invalid - there is no separate `invalid` prop to keep in step. */
  error?: ReactNode
  /** Keep the label for screen readers but take it off the screen. For a
   * field whose meaning is obvious in context - a search box in a toolbar. */
  labelHidden?: boolean
  className?: string
  /** Marks the field required, which is a statement about the form rather
   * than about validation: the label gets the mark a reader looks for. */
  required?: boolean
  /** The control's `name`, forwarded so a `Form` can attach a server error to
   * this field by name. */
  name?: string
  disabled?: boolean
}

export function Field({
  label,
  children,
  help,
  error,
  labelHidden = false,
  className,
  required = false,
  name,
  disabled = false,
}: FieldProps) {
  const invalid = error !== undefined && error !== null && error !== false

  return (
    <Base.Root
      name={name}
      disabled={disabled}
      // Base UI marks the control invalid from its own validity state; this
      // says so for an error that arrived from anywhere else.
      invalid={invalid || undefined}
      className={cn('flex flex-col gap-1.5', className)}
    >
      <Base.Label
        className={cn(
          'caption',
          // Off the screen, still in the accessibility tree. Not
          // `display: none`, which would take it out of both.
          labelHidden && 'sr-only',
        )}
      >
        {label}
        {required && (
          <span aria-hidden className="ml-0.5 text-bad">
            *
          </span>
        )}
      </Base.Label>

      {/* The control is handed to Base UI rather than merely nested inside
        * it. This is the whole component: a plain child renders a label whose
        * `for` points at an id nothing carries, so the label looks wired and
        * names nothing - which is exactly the bug this exists to prevent, and
        * it is invisible in a screenshot. `render` gives the child the id,
        * `aria-labelledby` and `aria-describedby` instead. */}
      <Base.Control render={children} />

      {/* The error wins the one line under the control. `match` is not used:
        * the string is already the decision - this renders whatever it was
        * handed rather than asking the browser what is wrong. */}
      {invalid ? (
        <Base.Error className="text-xs text-bad" match>
          {error}
        </Base.Error>
      ) : (
        help !== undefined && <Base.Description className="text-xs text-dim">{help}</Base.Description>
      )}
    </Base.Root>
  )
}

export interface FieldGroupProps {
  /** What the group is called - "Type", "References". Always rendered;
   * `labelHidden` only takes it off the screen. */
  label: ReactNode
  /** The controls: a row of chips, a segment, a grid of pictures, several
   * checkboxes. Anything - a group names its contents rather than handing
   * them an id. */
  children: ReactNode
  /** A hint under the group, read as the group's description. Hidden while
   * an error is showing, as in Field. */
  help?: ReactNode
  /** What is wrong with the group as a whole - "choose at least one". */
  error?: ReactNode
  labelHidden?: boolean
  required?: boolean
  /** Disables every native control inside, which is what a `<fieldset>`
   * does on its own. */
  disabled?: boolean
  className?: string
}

/**
 * FieldGroup - one name for several controls.
 *
 * Field labels exactly one control, and it has to: the label points at an id
 * the control carries. A row of chips, a segment or a grid of pictures has no
 * single control to point at, and the obvious workaround - a `<label>`
 * wrapped around the row - is a trap. A label with no `for` labels the first
 * labelable element inside it, and a click on any plain part of it is
 * forwarded there. kilna measured what that does in its style dialog: a click
 * on the caption "Type" silently set the first type, and a click on
 * "References" pressed the hidden remove cross of the first picture, which
 * deleted it for good.
 *
 * So a group is a `<fieldset>` with a legend, through Base UI's Fieldset: the
 * group is announced by its name as a reader enters it, and a caption has
 * nothing a click could be forwarded to. The caption is the same `caption` a
 * Field's label is, so the two kinds of field sit in one form as one kind.
 */
export function FieldGroup({
  label,
  children,
  help,
  error,
  labelHidden = false,
  required = false,
  disabled = false,
  className,
}: FieldGroupProps) {
  const noteId = useId()
  const invalid = error !== undefined && error !== null && error !== false
  const note = invalid ? error : help

  return (
    <Fieldset.Root
      disabled={disabled}
      aria-describedby={note === undefined ? undefined : noteId}
      data-invalid={invalid ? '' : undefined}
      // `min-w-0`: a fieldset's intrinsic minimum is its content, so a row of
      // chips inside a grid cell refuses to wrap and pushes the grid apart.
      className={cn('m-0 flex min-w-0 flex-col gap-1.5 border-0 p-0', className)}
    >
      <Fieldset.Legend className={cn('caption', labelHidden && 'sr-only')}>
        {label}
        {required && (
          <span aria-hidden className="ml-0.5 text-bad">
            *
          </span>
        )}
      </Fieldset.Legend>

      {children}

      {note === undefined ? null : (
        <span id={noteId} className={cn('text-xs', invalid ? 'text-bad' : 'text-dim')}>
          {note}
        </span>
      )}
    </Fieldset.Root>
  )
}
