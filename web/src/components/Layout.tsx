import { Link as RouterLink, Outlet } from 'react-router'
import { useAuth } from '../lib/useAuth'

export function Wordmark() {
  return (
    <span className="tape w-fit text-lg" aria-label="snip">
      snip
    </span>
  )
}

export function Layout() {
  const { user, signOut } = useAuth()

  return (
    <div className="min-h-dvh">
      <header className="border-b border-line bg-paper">
        <div className="mx-auto flex max-w-5xl items-center justify-between gap-4 px-5 py-3">
          <RouterLink to="/" className="rounded-sm">
            <Wordmark />
          </RouterLink>
          <div className="flex items-center gap-4 text-sm">
            <span className="hidden text-muted sm:inline">{user?.email}</span>
            <button type="button" onClick={signOut} className="font-medium text-ink hover:text-signal">
              Sign out
            </button>
          </div>
        </div>
      </header>
      <main className="mx-auto max-w-5xl px-5 py-8 sm:py-12">
        <Outlet />
      </main>
    </div>
  )
}
