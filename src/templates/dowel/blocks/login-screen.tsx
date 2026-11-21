import { useRef, useState, type FormEvent, type ReactNode } from 'react'
import { cn } from 'dowel-ui'
import { Alert } from '@/components/ui/alert'
import { Button } from '@/components/ui/button'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Panel } from '@/components/ui/panel'
import { PasswordField } from '@/components/ui/password-field'

/*
 * LoginScreen.
 *
 * The page a product opens on for someone who is not signed in yet: a name,
 * a password, a button, and the two ways it can go wrong said differently -
 * the server refused, or the server could not be reached.
 *
 * Every product of the line with a server drew this, and every one drew it
 * slightly wrong in the same few places. The fields zoomed a phone's page on
 * focus (the set's fields are 16px under a finger now, so this one is too).
 * A refusal and a dead network both said "wrong password", which sends
 * someone whose password is right to reset it. The password manager filled
 * the wrong box, because the name field was not marked `username`. And a
 * second press while the first was still on its way signed in twice.
 *
 * What goes wrong is the product's to say, so `onSubmit` says it: resolve
 * with nothing and the product moves on; resolve with words and they are
 * shown, the password selected so the next attempt types over it; reject,
 * and the screen says `words.failed` - the server is out of reach, which is
 * a different thing from being refused and should read as one.
 *
 * It is the page, so it is the page's `main`: render it outside the frame of
 * the signed-in product, where there is no rail to sit beside.
 */

export interface LoginCredentials {
  /** The email or the user name, as typed. */
  identifier: string
  password: string
}

/** Every word the screen shows, in the product's language. */
export interface LoginScreenWords {
  /** The first field's label: "Email", or "User name". */
  identifier: string
  /** "Password". */
  password: string
  /** What the reveal button is called while the password is hidden, and while it shows. */
  show: string
  hide: string
  /** The button: "Sign in". */
  submit: ReactNode
  /** The button while the attempt is on its way: "Signing in…". */
  submitting: ReactNode
  /** Said when the attempt never got an answer: "The server cannot be
   * reached. Try again in a moment." */
  failed: ReactNode
}

export interface LoginScreenProps {
  /** "Sign in to kilna". */
  title: ReactNode
  /** A line under it, when the title does not say enough. */
  description?: ReactNode
  /** The product's mark, above the title: `<ProductMark product="kilna" size={40} />`. */
  mark?: ReactNode
  /** What the first field asks for. An email gets the keyboard and the
   * check a browser gives an address; a user name gets neither. */
  identifier?: 'email' | 'username'
  /** Resolve with nothing when signed in; with words when refused - "That
   * email and password do not match." Reject when there was no answer. */
  onSubmit: (credentials: LoginCredentials) => Promise<ReactNode | void>
  /** Under the password: "Forgot your password?" */
  extra?: ReactNode
  /** Under the panel: "No account? Ask whoever runs this server." */
  footer?: ReactNode
  /** Every string the screen shows: the two field labels, the show and hide
   * names of the password toggle, the submit button in both states, and the
   * message for a server that could not be reached. The block has no words of
   * its own. */
  words: LoginScreenWords
  className?: string
}

export function LoginScreen({
  title,
  description,
  mark,
  identifier = 'email',
  onSubmit,
  extra,
  footer,
  words,
  className,
}: LoginScreenProps) {
  const [name, setName] = useState('')
  const [password, setPassword] = useState('')
  const [problem, setProblem] = useState<ReactNode>(null)
  const [pending, setPending] = useState(false)
  const passwordRef = useRef<HTMLInputElement>(null)

  async function submit(event: FormEvent) {
    event.preventDefault()
    // A second press while the first is on its way would sign in twice.
    if (pending) return
    setProblem(null)
    setPending(true)
    let refusal: ReactNode
    try {
      const answer = await onSubmit({ identifier: name, password })
      // Nothing, null or false is a yes; anything else is the reason for no.
      refusal = answer === undefined || answer === null || answer === false ? null : answer
    } catch {
      refusal = words.failed
    }
    setPending(false)
    if (refusal === null) return
    setProblem(refusal)
    // The next attempt types over the password rather than after it.
    passwordRef.current?.focus()
    passwordRef.current?.select()
  }

  return (
    <main className={cn('flex min-h-full flex-1 flex-col items-center justify-center gap-4 p-4', className)}>
      <Panel className="flex w-full max-w-sm flex-col gap-6 p-6">
        <div className="flex flex-col items-start gap-3">
          {mark}
          <div className="flex flex-col gap-1">
            <h1 className="text-lg font-semibold text-text">{title}</h1>
            {description === undefined ? null : <p className="text-sm text-dim">{description}</p>}
          </div>
        </div>
        <form onSubmit={(event) => void submit(event)} className="flex flex-col gap-4">
          <Field label={words.identifier} required>
            <Input
              type={identifier === 'email' ? 'email' : 'text'}
              // `username` either way: it is the token a password manager
              // pairs with `current-password` to know which box is which.
              autoComplete="username"
              autoCapitalize="none"
              spellCheck={false}
              required
              value={name}
              onChange={(event) => setName(event.target.value)}
            />
          </Field>
          <Field label={words.password} required>
            <PasswordField
              ref={passwordRef}
              autoComplete="current-password"
              showLabel={words.show}
              hideLabel={words.hide}
              required
              value={password}
              onValueChange={setPassword}
            />
          </Field>
          {extra === undefined ? null : <div className="-mt-2 text-sm">{extra}</div>}
          {/* `role="alert"`, so the refusal is announced and not only drawn:
              the focus has gone to the password, and a reader on it would
              otherwise hear nothing about why. */}
          {problem === null ? null : (
            <Alert tone="bad" role="alert">
              {problem}
            </Alert>
          )}
          <Button type="submit" variant="primary" disabled={pending} aria-busy={pending || undefined}>
            {pending ? words.submitting : words.submit}
          </Button>
        </form>
      </Panel>
      {footer === undefined ? null : <div className="text-sm text-dim">{footer}</div>}
    </main>
  )
}
