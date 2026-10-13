import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import type { FormEvent } from 'react'
import { Link as RouterLink, useNavigate, useParams } from 'react-router'
import { CopyButton, ErrorText, LinkStatus, Tape } from '../components/ui'
import { api } from '../lib/api'
import type { Link, LinkStats } from '../lib/api'
import { displayShortUrl, displayUrl, formatDate, isExpired, plural, timeAgo } from '../lib/format'

export function LinkPage() {
  const id = Number(useParams().id)
  const link = useQuery({ queryKey: ['link', id], queryFn: () => api.getLink(id) })
  const stats = useQuery({ queryKey: ['link', id, 'stats'], queryFn: () => api.linkStats(id) })

  if (link.isPending) return <p className="py-12 text-center text-muted">Loading…</p>
  if (link.isError) {
    return (
      <div className="space-y-4">
        <BackLink />
        <ErrorText error={link.error} />
      </div>
    )
  }

  return (
    <div className="space-y-12">
      <div className="space-y-6">
        <div className="pb-2">
          <BackLink />
        </div>
        <div className="flex flex-wrap items-center gap-x-6 gap-y-4">
          <h1>
            <Tape code={link.data.code} hero />
          </h1>
          <LinkStatus link={link.data} />
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <a href={link.data.short_url} target="_blank" rel="noreferrer" className="font-mono text-signal hover:underline">
            {displayShortUrl(link.data.short_url)}
          </a>
          <CopyButton text={link.data.short_url} className="btn-quiet py-1 text-sm" />
        </div>
      </div>

      <ClicksSection stats={stats.data} error={stats.error} />

      <div className="grid gap-12 lg:grid-cols-[1fr_20rem]">
        <TargetEditor link={link.data} />
        <Referrers stats={stats.data} />
      </div>

      <DangerZone link={link.data} />
    </div>
  )
}

function BackLink() {
  return (
    <RouterLink to="/" className="text-sm font-medium text-muted hover:text-ink">
      ← All links
    </RouterLink>
  )
}

/** Fills in the days without clicks so the chart always shows 30 bars. */
function last30Days(daily: LinkStats['daily']) {
  const byDate = new Map(daily.map((d) => [d.date, d.clicks]))
  const days: { date: string; clicks: number }[] = []
  const today = new Date()
  for (let i = 29; i >= 0; i--) {
    const d = new Date(Date.UTC(today.getUTCFullYear(), today.getUTCMonth(), today.getUTCDate() - i))
    const key = d.toISOString().slice(0, 10)
    days.push({ date: key, clicks: byDate.get(key) ?? 0 })
  }
  return days
}

function shortDay(isoDate: string) {
  return new Date(`${isoDate}T00:00:00Z`).toLocaleDateString('en', { month: 'short', day: 'numeric', timeZone: 'UTC' })
}

function ClicksSection({ stats, error }: { stats: LinkStats | undefined; error: unknown }) {
  if (error) return <ErrorText error={error} />
  if (!stats) return <div className="h-56" />

  const days = last30Days(stats.daily)
  const max = Math.max(1, ...days.map((d) => d.clicks))
  const lastMonth = days.reduce((sum, d) => sum + d.clicks, 0)
  const best = days.reduce((a, b) => (b.clicks > a.clicks ? b : a))

  return (
    <section aria-labelledby="clicks-heading" className="space-y-6">
      <div className="flex flex-wrap items-end justify-between gap-x-10 gap-y-4">
        <h2 id="clicks-heading" className="text-xl font-semibold">
          Clicks
        </h2>
        <dl className="grid w-full grid-cols-3 gap-4 sm:flex sm:w-auto sm:gap-10">
          <Stat label="All time" value={stats.total_clicks.toLocaleString('en')} />
          <Stat label="Last 30 days" value={lastMonth.toLocaleString('en')} />
          <Stat label="Last click" value={stats.last_clicked_at ? timeAgo(stats.last_clicked_at) : 'Never'} />
        </dl>
      </div>

      <div className="rounded-md border border-line bg-paper p-4 sm:p-6">
        <div
          role="img"
          aria-label={
            lastMonth === 0
              ? 'No clicks in the last 30 days.'
              : `${plural(lastMonth, 'click')} in the last 30 days. Busiest day: ${shortDay(best.date)} with ${plural(best.clicks, 'click')}.`
          }
          className="flex h-40 items-end gap-[3px]"
        >
          {days.map((d) => (
            <div
              key={d.date}
              title={`${shortDay(d.date)}: ${plural(d.clicks, 'click')}`}
              className={`flex-1 rounded-t-sm ${d.clicks > 0 ? 'bg-signal hover:bg-signal-dark' : 'bg-line/60'}`}
              style={{ height: d.clicks > 0 ? `${Math.max(4, (d.clicks / max) * 100)}%` : '2px' }}
            />
          ))}
        </div>
        <div className="mt-2 flex justify-between text-xs text-muted">
          <span>{shortDay(days[0].date)}</span>
          <span>Today</span>
        </div>
        {lastMonth === 0 && (
          <p className="mt-4 text-sm text-muted">No clicks in the last 30 days. Share the short link to start collecting them.</p>
        )}
      </div>
    </section>
  )
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt className="text-sm whitespace-nowrap text-muted">{label}</dt>
      <dd className="text-lg font-semibold tabular-nums sm:text-2xl">{value}</dd>
    </div>
  )
}

function Referrers({ stats }: { stats: LinkStats | undefined }) {
  if (!stats) return null
  const total = stats.top_referrers.reduce((s, r) => s + r.clicks, 0)

  return (
    <section aria-labelledby="ref-heading" className="space-y-4">
      <h2 id="ref-heading" className="text-xl font-semibold">
        Where clicks come from
      </h2>
      {stats.top_referrers.length === 0 ? (
        <p className="text-sm text-muted">Nothing yet. Sources show up after the first click.</p>
      ) : (
        <ul className="space-y-3">
          {stats.top_referrers.map((r) => (
            <li key={r.referrer ?? 'direct'} className="space-y-1">
              <div className="flex justify-between gap-3 text-sm">
                <span className="truncate">{r.referrer ? displayUrl(r.referrer) : 'Direct or unknown'}</span>
                <span className="font-medium tabular-nums">{r.clicks.toLocaleString('en')}</span>
              </div>
              <div className="h-1.5 overflow-hidden rounded-full bg-line/60">
                <div className="h-full bg-signal" style={{ width: `${(r.clicks / total) * 100}%` }} />
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}

function useUpdateLink(link: Link) {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (input: Parameters<typeof api.updateLink>[1]) => api.updateLink(link.id, input),
    onSuccess: (updated) => {
      queryClient.setQueryData(['link', link.id], updated)
      queryClient.invalidateQueries({ queryKey: ['links'] })
    },
  })
}

function TargetEditor({ link }: { link: Link }) {
  const [editing, setEditing] = useState(false)
  const update = useUpdateLink(link)

  function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault()
    const target = String(new FormData(e.currentTarget).get('target_url')).trim()
    update.mutate({ target_url: target }, { onSuccess: () => setEditing(false) })
  }

  return (
    <section aria-labelledby="target-heading" className="space-y-4">
      <h2 id="target-heading" className="text-xl font-semibold">
        Destination
      </h2>
      {editing ? (
        <form onSubmit={onSubmit} className="space-y-3">
          <label className="block">
            <span className="sr-only">Destination URL</span>
            <input name="target_url" type="url" required defaultValue={link.target_url} className="field font-mono text-sm" autoFocus />
          </label>
          <ErrorText error={update.error} />
          <div className="flex gap-3">
            <button type="submit" disabled={update.isPending} className="btn-primary py-2">
              {update.isPending ? 'Saving…' : 'Save destination'}
            </button>
            <button
              type="button"
              className="btn-quiet py-2"
              onClick={() => {
                setEditing(false)
                update.reset()
              }}
            >
              Cancel
            </button>
          </div>
        </form>
      ) : (
        <div className="space-y-3">
          <a href={link.target_url} target="_blank" rel="noreferrer" className="block font-mono text-sm break-all hover:text-signal">
            {link.target_url}
          </a>
          <p className="text-sm text-muted">
            Created {formatDate(link.created_at)}
            {link.expires_at && (isExpired(link.expires_at) ? `, expired ${formatDate(link.expires_at)}` : `, expires ${formatDate(link.expires_at)}`)}
            .
          </p>
          <button type="button" onClick={() => setEditing(true)} className="btn-quiet py-2 text-sm">
            Change destination
          </button>
        </div>
      )}
    </section>
  )
}

function DangerZone({ link }: { link: Link }) {
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const update = useUpdateLink(link)
  const [confirming, setConfirming] = useState(false)

  const remove = useMutation({
    mutationFn: () => api.deleteLink(link.id),
    onSuccess: () => {
      queryClient.removeQueries({ queryKey: ['link', link.id] })
      queryClient.invalidateQueries({ queryKey: ['links'] })
      navigate('/')
    },
  })

  return (
    <section aria-labelledby="manage-heading" className="space-y-4 border-t border-line pt-8">
      <h2 id="manage-heading" className="text-xl font-semibold">
        Manage
      </h2>
      <div className="flex flex-wrap items-start gap-x-8 gap-y-6">
        <div className="max-w-xs space-y-2">
          <button
            type="button"
            disabled={update.isPending}
            onClick={() => update.mutate({ is_active: !link.is_active })}
            className="btn-quiet"
          >
            {link.is_active ? 'Pause link' : 'Resume link'}
          </button>
          <p className="text-sm text-muted">
            {link.is_active
              ? `Visitors to ${displayShortUrl(link.short_url)} will see a "link unavailable" error until you resume it.`
              : 'This link is paused. Resume it to start redirecting again.'}
          </p>
        </div>

        <div className="max-w-xs space-y-2">
          {confirming ? (
            <div className="flex gap-3">
              <button type="button" disabled={remove.isPending} onClick={() => remove.mutate()} className="btn-danger bg-danger text-white">
                {remove.isPending ? 'Deleting…' : 'Yes, delete it'}
              </button>
              <button type="button" onClick={() => setConfirming(false)} className="btn-quiet">
                Keep it
              </button>
            </div>
          ) : (
            <button type="button" onClick={() => setConfirming(true)} className="btn-danger">
              Delete link
            </button>
          )}
          <p className="text-sm text-muted">Deleting removes the link and its {plural(link.click_count, 'recorded click')}. This can't be undone.</p>
          <ErrorText error={remove.error ?? update.error} />
        </div>
      </div>
    </section>
  )
}
