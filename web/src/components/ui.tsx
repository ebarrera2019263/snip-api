import { useEffect, useState } from 'react'
import type { Link } from '../lib/api'
import { isExpired } from '../lib/format'

export function Tape({ code, hero = false }: { code: string; hero?: boolean }) {
  return <span className={hero ? 'tape tape-hero' : 'tape text-sm'}>{code}</span>
}

export function CopyButton({ text, className = 'btn-quiet' }: { text: string; className?: string }) {
  const [copied, setCopied] = useState(false)

  useEffect(() => {
    if (!copied) return
    const timer = setTimeout(() => setCopied(false), 1600)
    return () => clearTimeout(timer)
  }, [copied])

  return (
    <button
      type="button"
      className={`${className} min-w-[5.5rem]`}
      onClick={(e) => {
        // Rows are clickable; copying shouldn't also open the link.
        e.stopPropagation()
        navigator.clipboard.writeText(text).then(() => setCopied(true))
      }}
    >
      <span aria-live="polite">{copied ? 'Copied' : 'Copy'}</span>
    </button>
  )
}

/** Shown only when a link won't redirect; active links need no badge. */
export function LinkStatus({ link }: { link: Link }) {
  if (isExpired(link.expires_at)) {
    return <span className="rounded bg-danger/10 px-2 py-0.5 text-xs font-medium text-danger">Expired</span>
  }
  if (!link.is_active) {
    return <span className="rounded bg-ink/10 px-2 py-0.5 text-xs font-medium text-muted">Paused</span>
  }
  return null
}

export function ErrorText({ error }: { error: unknown }) {
  if (!error) return null
  const message = error instanceof Error ? error.message : 'Something went wrong.'
  return (
    <p role="alert" className="text-sm text-danger">
      {message.charAt(0).toUpperCase() + message.slice(1)}
    </p>
  )
}
