import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useCallback, useEffect, useMemo, useState } from 'react'
import type { ReactNode } from 'react'
import { api, setUnauthorizedHandler, tokenStore } from './api'
import type { AuthResponse } from './api'
import { AuthContext } from './useAuth'

export function AuthProvider({ children }: { children: ReactNode }) {
  const queryClient = useQueryClient()
  const [token, setToken] = useState(tokenStore.get)

  const signOut = useCallback(() => {
    tokenStore.clear()
    setToken(null)
    queryClient.clear()
  }, [queryClient])

  const signIn = useCallback(
    ({ token, user }: AuthResponse) => {
      tokenStore.set(token)
      queryClient.setQueryData(['me'], user)
      setToken(token)
    },
    [queryClient],
  )

  // Expired or revoked tokens: any 401 signs the user out.
  useEffect(() => setUnauthorizedHandler(signOut), [signOut])

  const me = useQuery({
    queryKey: ['me'],
    queryFn: api.me,
    enabled: token !== null,
    retry: false,
  })

  const value = useMemo(
    () => ({
      user: token ? me.data : undefined,
      isLoading: token !== null && me.isPending,
      signIn,
      signOut,
    }),
    [token, me.data, me.isPending, signIn, signOut],
  )

  return <AuthContext value={value}>{children}</AuthContext>
}
