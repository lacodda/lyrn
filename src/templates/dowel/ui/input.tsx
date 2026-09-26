import type { InputHTMLAttributes, Ref } from 'react'
import { cn } from 'dowel-ui'

/*
 * Input.
 *
 * A single-line field. It is a plain `<input>` with the line's clothes on, so
 * everything a browser gives an input for free - autofill, spellcheck, the
 * right keyboard on a phone, `type="email"` validation - still works.
 *
 * The focus ring is the accent, drawn outside the border rather than replacing
 * it: a field that only changes colour on focus is invisible to anyone who
 * cannot distinguish those two colours.
 */
export const fieldClasses = cn(
  'w-full rounded-md border border-line bg-transparent px-2.5 py-1.5',
  'text-sm text-text placeholder:text-faint',
  'transition-colors hover:border-line-2',
  'focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-accent',
  // A field nobody can type in should look like one.
  'disabled:cursor-not-allowed disabled:opacity-50',
  // `aria-invalid` rather than a prop: the attribute is what a screen reader
  // reads, so making it the source of the colour keeps the two in step.
  'aria-invalid:border-bad aria-invalid:focus-visible:outline-bad',
)

export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  /** React 19 passes `ref` as a plain prop; it is declared so callers can
   * reach the element to focus it or read its selection. */
  ref?: Ref<HTMLInputElement>
}

export function Input({ className, ref, ...props }: InputProps) {
  return <input ref={ref} className={cn(fieldClasses, 'h-control', className)} {...props} />
}
