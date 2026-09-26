/*
 * The server, as this application calls it.
 *
 * The session lives in an HttpOnly cookie that the page cannot read, which is
 * the point: a script injected into the page cannot steal it either. So every
 * call is same-origin, and the only way to know who is signed in is to ask
 * (`me`). In development Vite forwards `/api` to the service - see
 * `vite.config.ts`.
 */

/** A call the server answered with anything but success. */
export class ApiError extends Error {
  readonly status: number

  constructor(status: number, message: string) {
    super(message)
    this.name = 'ApiError'
    this.status = status
  }

  /** Nobody is signed in, or the session has ended. */
  get isUnauthorized(): boolean {
    return this.status === 401
  }
}

/** Who is signed in. */
export interface Identity {
  email: string
  name: string
}

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const response = await fetch(path, {
    ...init,
    credentials: 'same-origin',
    headers: { ...(init.body ? { 'Content-Type': 'application/json' } : {}), ...init.headers },
  })
  if (!response.ok) {
    throw new ApiError(response.status, await reason(response))
  }
  if (response.status === 204) {
    return undefined as T
  }
  return (await response.json()) as T
}

/** What the server said went wrong: the line's services answer
 * `{ "error": "..." }`, and anything else falls back to the status text. */
async function reason(response: Response): Promise<string> {
  try {
    const body = (await response.json()) as { error?: unknown }
    if (typeof body.error === 'string') return body.error
  } catch {
    // Not JSON - a proxy's error page, say. The status says enough.
  }
  return response.statusText || `HTTP ${response.status}`
}

export const api = {
  me: () => request<Identity>('/api/me'),
  signIn: (email: string, password: string) =>
    request<void>('/api/auth/login', { method: 'POST', body: JSON.stringify({ email, password }) }),
  signOut: () => request<void>('/api/auth/logout', { method: 'POST' }),
}
