import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import type { FormEvent } from 'react'
import { useNavigate } from 'react-router'
import { CopyButton, ErrorText, LinkStatus, Tape } from '../components/ui'
import { api } from '../lib/api'
import type { Link } from '../lib/api'
import { displayShortUrl, displayUrl, plural, timeAgo } from '../lib/format'

const PER_PAGE = 10

export function LinksPage() {
  const [page, setPage] = useState(1)
  const [justCreated, setJustCreated] = useState<Link | null>(null)

  const links = useQuery({
    queryKey: ['links', page],
    queryFn: () => api.listLinks(page, PER_PAGE),
    placeholderData: keepPreviousData,
  })

  const total = links.data?.total ?? 0
  const pageCount = Math.max(1, Math.ceil(total / PER_PAGE))

  return (
    <div className="space-y-12">
      <ShortenForm
        onCreated={(link) => {
          setJustCreated(link)
          setPage(1)
        }}
      />

      {justCreated && (
        <div className="flex flex-wrap items-center gap-x-4 gap-y-3 rounded-md border border-signal/30 bg-signal/5 px-4 py-3">
          <span className="font-medium">Link created.</span>
          <span className="font-mono text-sm break-all">{displayShortUrl(justCreated.short_url)}</span>
          <CopyButton text={justCreated.short_url} className="btn-primary ml-auto py-1.5" />
        </div>
      )}

      <section aria-labelledby="links-heading" className="space-y-4">
        <div className="flex items-baseline justify-between gap-4">
          <h2 id="links-heading" className="text-xl font-semibold">
            Your links
          </h2>
          {total > 0 && <span className="text-sm text-muted">{plural(total, 'link')}</span>}
        </div>

        {links.isPending ? (
          <p className="py-12 text-center text-muted">Loading your links…</p>
        ) : links.isError ? (
          <ErrorText error={links.error} />
        ) : total === 0 ? (
          <div className="rounded-md border border-dashed border-line px-6 py-14 text-center">
            <p className="font-medium">No links yet.</p>
            <p className="mt-1 text-muted">Paste a URL above to create your first one.</p>
          </div>
        ) : (
          <>
            <ul className="divide-y divide-line overflow-hidden rounded-md border border-line bg-paper">
              {links.data.items.map((link) => (
                <LinkRow key={link.id} link={link} highlight={link.id === justCreated?.id} />
              ))}
            </ul>
            {pageCount > 1 && (
              <nav aria-label="Pagination" className="flex items-center justify-between gap-4 text-sm">
                <button
                  type="button"
                  className="btn-quiet py-1.5"
                  disabled={page === 1}
                  onClick={() => setPage(page - 1)}
                >
                  Newer
                </button>
                <span className="text-muted">
                  Page {page} of {pageCount}
                </span>
                <button
                  type="button"
                  className="btn-quiet py-1.5"
                  disabled={page >= pageCount}
                  onClick={() => setPage(page + 1)}
                >
                  Older
                </button>
              </nav>
            )}
          </>
        )}
      </section>
    </div>
  )
}

function ShortenForm({ onCreated }: { onCreated: (link: Link) => void }) {
  const queryClient = useQueryClient()
  const [showCode, setShowCode] = useState(false)

  const create = useMutation({
    mutationFn: api.createLink,
    onSuccess: (link) => {
      queryClient.invalidateQueries({ queryKey: ['links'] })
      onCreated(link)
    },
  })

  function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault()
    const formEl = e.currentTarget
    const form = new FormData(formEl)
    const code = String(form.get('code') ?? '').trim()
    create.mutate(
      { target_url: String(form.get('target_url')).trim(), code: code || undefined },
      { onSuccess: () => formEl.reset() },
    )
  }

  return (
    <section aria-labelledby="shorten-heading">
      <h1 id="shorten-heading" className="text-3xl font-semibold sm:text-4xl">
        Shorten a link
      </h1>
      <form onSubmit={onSubmit} className="mt-6 space-y-3">
        <div className="flex flex-col gap-3 sm:flex-row">
          <label className="flex-1">
            <span className="sr-only">Long URL</span>
            <input
              name="target_url"
              type="url"
              required
              placeholder="Paste a long URL"
              className="field py-3 text-lg"
            />
          </label>
          <button type="submit" disabled={create.isPending} className="btn-primary px-6 py-3 text-lg">
            {create.isPending ? 'Shortening…' : 'Shorten'}
          </button>
        </div>

        {showCode ? (
          <label className="flex max-w-md items-center gap-3">
            <span className="shrink-0 text-sm font-medium">Custom code</span>
            <input
              name="code"
              pattern="[A-Za-z0-9_\-]{3,32}"
              title="3 to 32 letters, digits, - or _"
              placeholder="rust-book"
              className="field py-1.5 font-mono"
              autoFocus
            />
          </label>
        ) : (
          <button
            type="button"
            onClick={() => setShowCode(true)}
            className="text-sm font-medium text-signal hover:underline"
          >
            Choose your own code
          </button>
        )}
        <ErrorText error={create.error} />
      </form>
    </section>
  )
}

function LinkRow({ link, highlight }: { link: Link; highlight: boolean }) {
  const navigate = useNavigate()
  const open = () => navigate(`/links/${link.id}`)

  return (
    <li
      className={`grid cursor-pointer grid-cols-[1fr_auto] items-center gap-x-4 gap-y-2 px-4 py-4 hover:bg-fog/60 sm:grid-cols-[minmax(0,1fr)_6rem_auto] ${
        highlight ? 'bg-tape/10' : ''
      }`}
      onClick={open}
    >
      <div className="min-w-0 space-y-1.5">
        <div className="flex items-center gap-3">
          {/* The real link for keyboard and screen-reader users; the row click is a shortcut. */}
          <a
            href={`/links/${link.id}`}
            onClick={(e) => {
              e.preventDefault()
              e.stopPropagation()
              open()
            }}
            className="rounded-sm"
            aria-label={`${link.code}, details`}
          >
            <Tape code={link.code} />
          </a>
          <LinkStatus link={link} />
        </div>
        <p className="truncate text-sm text-muted" title={link.target_url}>
          {displayUrl(link.target_url)}
        </p>
        <p className="text-sm font-medium sm:hidden">{plural(link.click_count, 'click')}</p>
      </div>

      <div className="hidden text-right sm:block">
        <span className="block text-lg font-semibold tabular-nums">{link.click_count.toLocaleString('en')}</span>
        <span className="text-xs text-muted">{link.click_count === 1 ? 'click' : 'clicks'}</span>
      </div>

      <div className="flex flex-col items-end gap-1">
        <CopyButton text={link.short_url} className="btn-quiet py-1.5 text-sm" />
        <span className="text-xs text-muted">{timeAgo(link.created_at)}</span>
      </div>
    </li>
  )
}
