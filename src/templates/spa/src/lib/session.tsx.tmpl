import { createContext, use, useCallback, useEffect, useMemo, useState, type ReactNode } from 'react'
import { api, type Identity } from '@/lib/api'

interface Session {
  /** `undefined` while the first `me()` is still on its way. */
  user: Identity | null | undefined
  signIn: (email: string, password: string) => Promise<void>
  signOut: () => Promise<void>
}

const SessionContext = createContext<Session | null>(null)

/**
 * Who is signed in, asked of the server rather than remembered here.
 *
 * The cookie is out of the page's reach, so the only way to know whether it
 * is still good is to ask - which is also why a reload is not a sign-out: the
 * cookie survives it, and `me()` answers.
 */
export function SessionProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<Identity | null | undefined>(undefined)

  useEffect(() => {
    let cancelled = false
    api
      .me()
      .then((identity) => {
        if (!cancelled) setUser(identity)
      })
      .catch(() => {
        // A 401 is the ordinary "nobody is signed in", not a fault. A server
        // out of reach lands here too, and the sign-in form is still the
        // right place to be: it says so when a sign-in is tried.
        if (!cancelled) setUser(null)
      })
    return () => {
      cancelled = true
    }
  }, [])

  const signIn = useCallback(async (email: string, password: string) => {
    await api.signIn(email, password)
    // Signing in answers with a cookie, not a person: who it is comes from
    // the same call every reload makes.
    setUser(await api.me())
  }, [])

  const signOut = useCallback(async () => {
    try {
      await api.signOut()
    } finally {
      // Whatever the server said, this browser is done: leaving someone
      // apparently signed in after they asked to leave is the worse failure.
      setUser(null)
    }
  }, [])

  const value = useMemo(() => ({ user, signIn, signOut }), [user, signIn, signOut])
  return <SessionContext value={value}>{children}</SessionContext>
}

export function useSession(): Session {
  const session = use(SessionContext)
  if (!session) {
    throw new Error('useSession() is called outside <SessionProvider>')
  }
  return session
}
