import { useMutation } from '@tanstack/react-query'
import { useState } from 'react'
import type { FormEvent } from 'react'
import { Wordmark } from '../components/Layout'
import { ErrorText, Tape } from '../components/ui'
import { api } from '../lib/api'
import { useAuth } from '../lib/useAuth'

type Mode = 'login' | 'register'

export function AuthPage() {
  const { signIn } = useAuth()
  const [mode, setMode] = useState<Mode>('login')

  const submit = useMutation({
    mutationFn: ({ email, password }: { email: string; password: string }) =>
      mode === 'login' ? api.login(email, password) : api.register(email, password),
    onSuccess: signIn,
  })

  function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault()
    const form = new FormData(e.currentTarget)
    submit.mutate({ email: String(form.get('email')), password: String(form.get('password')) })
  }

  function switchMode() {
    setMode(mode === 'login' ? 'register' : 'login')
    submit.reset()
  }

  const isLogin = mode === 'login'

  return (
    <div className="grid min-h-dvh lg:grid-cols-[1.1fr_1fr]">
      <section className="flex flex-col justify-between gap-12 bg-ink px-6 py-8 text-fog sm:px-12 lg:py-12">
        <Wordmark />
        <div className="max-w-md">
          <p className="text-3xl leading-tight font-semibold sm:text-4xl">
            Turn long links into short ones, and see who clicks them.
          </p>
          <figure className="mt-10 space-y-4" aria-label="Example: a long URL shortened to rust-book">
            <p className="font-mono text-sm break-all text-fog/45 line-through decoration-fog/30">
              https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html
            </p>
            <Tape code="rust-book" hero />
          </figure>
          <ul className="mt-10 space-y-3 text-fog/75">
            <li>Pick your own code or get a random one.</li>
            <li>Track clicks per day and where they come from.</li>
            <li>Pause or redirect a link without changing it.</li>
          </ul>
        </div>
        <p className="text-sm text-fog/50">Rust + Axum API, React dashboard.</p>
      </section>

      <section className="flex items-center justify-center px-6 py-12">
        <form onSubmit={onSubmit} className="w-full max-w-sm space-y-5">
          <h1 className="text-2xl font-semibold">{isLogin ? 'Sign in' : 'Create an account'}</h1>

          <label className="block space-y-1.5">
            <span className="text-sm font-medium">Email</span>
            <input name="email" type="email" required autoComplete="email" className="field" />
          </label>

          <label className="block space-y-1.5">
            <span className="text-sm font-medium">Password</span>
            <input
              name="password"
              type="password"
              required
              minLength={8}
              autoComplete={isLogin ? 'current-password' : 'new-password'}
              className="field"
            />
            {!isLogin && <span className="block text-sm text-muted">At least 8 characters.</span>}
          </label>

          <ErrorText error={submit.error} />

          <button type="submit" disabled={submit.isPending} className="btn-primary w-full">
            {submit.isPending ? 'One moment…' : isLogin ? 'Sign in' : 'Create account'}
          </button>

          <p className="text-sm text-muted">
            {isLogin ? 'New here? ' : 'Already have an account? '}
            <button type="button" onClick={switchMode} className="font-medium text-signal hover:underline">
              {isLogin ? 'Create an account' : 'Sign in'}
            </button>
          </p>
        </form>
      </section>
    </div>
  )
}
