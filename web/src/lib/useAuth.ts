import { createContext, useContext } from 'react'
import type { AuthResponse, User } from './api'

export type AuthContextValue = {
  user: User | undefined
  isLoading: boolean
  signIn: (auth: AuthResponse) => void
  signOut: () => void
}

export const AuthContext = createContext<AuthContextValue | null>(null)

export function useAuth() {
  const ctx = useContext(AuthContext)
  if (!ctx) throw new Error('useAuth must be used inside <AuthProvider>')
  return ctx
}
