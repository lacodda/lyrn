{{#router}}
import { Navigate, Route, Routes } from 'react-router'
{{/router}}
{{#auth}}
import { Button } from '@/components/ui/button'
import { useSession } from '@/lib/session'
import { SignIn } from '@/pages/SignIn'
{{/auth}}
import { Home } from '@/pages/Home'

/** Which screen is showing. */
export function App() {
{{#auth}}
  const { user, signOut } = useSession()
  // The session is asked of the server on every load; until it answers there
  // is nothing true to show - not the sign-in form, not the application.
  if (user === undefined) return null
  if (user === null) return <SignIn />

  return (
    <>
      <header className="absolute top-0 right-0 flex items-center gap-3 p-4 text-sm text-dim">
        <span>{user.name}</span>
        <Button variant="link" onClick={() => void signOut()}>
          Sign out
        </Button>
      </header>
      <Screens />
    </>
  )
}

/** The screens someone signed in can reach. */
function Screens() {
{{/auth}}
{{#router}}
  return (
    <Routes>
      <Route path="/" element={<Home />} />
      {/* An address nothing answers goes home rather than to a blank page. */}
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  )
{{/router}}
{{^router}}
  return <Home />
{{/router}}
}
