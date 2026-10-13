import { Navigate, Route, Routes } from 'react-router'
import { Layout } from './components/Layout'
import { useAuth } from './lib/useAuth'
import { AuthPage } from './pages/AuthPage'
import { LinkPage } from './pages/LinkPage'
import { LinksPage } from './pages/LinksPage'

export default function App() {
  const { user, isLoading } = useAuth()

  if (isLoading) return null

  if (!user) {
    return (
      <Routes>
        <Route path="*" element={<AuthPage />} />
      </Routes>
    )
  }

  return (
    <Routes>
      <Route element={<Layout />}>
        <Route index element={<LinksPage />} />
        <Route path="links/:id" element={<LinkPage />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  )
}
