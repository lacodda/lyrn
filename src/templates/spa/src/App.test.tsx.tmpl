import { render, screen } from '@testing-library/react'
{{#router}}
import { MemoryRouter } from 'react-router'
{{/router}}
{{#auth}}
import { beforeEach, describe, expect, it } from 'vitest'
{{/auth}}
{{^auth}}
import { describe, expect, it } from 'vitest'
{{/auth}}
import { App } from '@/App'
{{#auth}}
import { signedInAs, signedOut, withSession } from '@/test/session'

// Signed in unless a test says otherwise: most of what the app does is for
// someone who is.
beforeEach(() => {
  signedInAs({ email: 'ada@example.com', name: 'Ada' })
})
{{/auth}}

{{#router}}
function renderApp(address = '/') {
  const app = (
    <MemoryRouter initialEntries={[address]}>
      <App />
    </MemoryRouter>
  )
{{/router}}
{{^router}}
function renderApp() {
  const app = <App />
{{/router}}
{{#auth}}
  render(withSession(app))
{{/auth}}
{{^auth}}
  render(app)
{{/auth}}
}

describe('App', () => {
  it('opens on the home screen', async () => {
    renderApp()
    expect(await screen.findByRole('heading', { name: '{{ title }}' })).toBeTruthy()
  })
{{#router}}

  it('sends an address nothing answers home', async () => {
    renderApp('/nowhere')
    expect(await screen.findByRole('heading', { name: '{{ title }}' })).toBeTruthy()
  })
{{/router}}
{{#auth}}

  it('says who is signed in', async () => {
    renderApp()
    expect(await screen.findByText('Ada')).toBeTruthy()
  })

  it('asks someone signed out to sign in', async () => {
    signedOut()
    renderApp()
    expect(await screen.findByRole('button', { name: 'Sign in' })).toBeTruthy()
    expect(screen.queryByRole('heading', { name: '{{ title }}' })).toBeNull()
  })
{{/auth}}
})
