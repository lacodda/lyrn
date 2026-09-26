import type { ReactNode } from 'react'
import { afterEach, vi } from 'vitest'
import type { Identity } from '@/lib/api'
import { SessionProvider } from '@/lib/session'

/*
 * The server, for the length of one test.
 *
 * The session is asked of the server, so a test that needs somebody signed in
 * says what the server answers rather than reaching into the session's state:
 * the provider, the API client and the screens are then the real ones.
 */

afterEach(() => {
  vi.unstubAllGlobals()
})

type Answer = () => Response | Promise<Response>

/** Answer each path with its own response; any other path is a failure. */
export function serve(routes: Record<string, Answer>) {
  vi.stubGlobal(
    'fetch',
    vi.fn(async (input: RequestInfo | URL) => {
      const path = typeof input === 'string' ? input : input instanceof URL ? input.pathname : input.url
      const answer = routes[path]
      if (!answer) throw new TypeError(`the test server has no answer for ${path}`)
      return answer()
    }),
  )
}

export const unauthorized: Answer = () => Response.json({ error: 'not signed in' }, { status: 401 })
export const noContent: Answer = () => new Response(null, { status: 204 })

export function signedInAs(identity: Identity) {
  serve({ '/api/me': () => Response.json(identity), '/api/auth/logout': noContent })
}

export function signedOut() {
  serve({ '/api/me': unauthorized })
}

export function withSession(ui: ReactNode) {
  return <SessionProvider>{ui}</SessionProvider>
}
