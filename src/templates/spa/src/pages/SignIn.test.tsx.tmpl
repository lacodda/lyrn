import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { SignIn } from '@/pages/SignIn'
import { noContent, serve, unauthorized, withSession } from '@/test/session'

function signInAs(email: string, password: string) {
  render(withSession(<SignIn />))
  fireEvent.change(screen.getByLabelText(/email/i), { target: { value: email } })
  fireEvent.change(screen.getByLabelText(/password/i), { target: { value: password } })
  fireEvent.click(screen.getByRole('button', { name: 'Sign in' }))
}

describe('SignIn', () => {
  it('says a wrong password is wrong', async () => {
    serve({ '/api/me': unauthorized, '/api/auth/login': unauthorized })
    signInAs('ada@example.com', 'wrong')
    expect((await screen.findByRole('alert')).textContent).toMatch(/do not match/)
  })

  it('says a server out of reach is out of reach, not that the password is wrong', async () => {
    serve({
      '/api/me': unauthorized,
      '/api/auth/login': () => {
        throw new TypeError('Failed to fetch')
      },
    })
    signInAs('ada@example.com', 'right')
    expect((await screen.findByRole('alert')).textContent).toMatch(/cannot be reached/)
  })

  it('sends the address and the password, and nothing else', async () => {
    let signedIn = false
    serve({
      '/api/me': () => (signedIn ? Response.json({ email: 'ada@example.com', name: 'Ada' }) : unauthorized()),
      '/api/auth/login': () => {
        signedIn = true
        return noContent()
      },
    })
    signInAs('ada@example.com', 'right')
    await waitFor(() => expect(signedIn).toBe(true))
    const login = vi.mocked(fetch).mock.calls.find(([path]) => path === '/api/auth/login')
    expect(JSON.parse(String(login?.[1]?.body))).toEqual({ email: 'ada@example.com', password: 'right' })
  })
})
