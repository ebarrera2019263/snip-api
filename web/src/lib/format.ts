const relative = new Intl.RelativeTimeFormat('en', { numeric: 'auto' })

const UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
  ['year', 365 * 24 * 3600],
  ['month', 30 * 24 * 3600],
  ['day', 24 * 3600],
  ['hour', 3600],
  ['minute', 60],
]

/** "3 hours ago", "yesterday", "just now". */
export function timeAgo(iso: string, now = Date.now()): string {
  const seconds = (new Date(iso).getTime() - now) / 1000
  for (const [unit, size] of UNITS) {
    if (Math.abs(seconds) >= size) return relative.format(Math.round(seconds / size), unit)
  }
  return 'just now'
}

/** In UTC, matching the days the API groups clicks by in the stats chart. */
export function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString('en', {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
    timeZone: 'UTC',
  })
}

/** "doc.rust-lang.org/book" — the readable part of a URL. */
export function displayUrl(raw: string): string {
  try {
    const url = new URL(raw)
    const path = url.pathname === '/' ? '' : url.pathname.replace(/\/$/, '')
    return `${url.host.replace(/^www\./, '')}${path}${url.search}`
  } catch {
    return raw
  }
}

/** "localhost:8080/rust-book" — short URL without the scheme. */
export function displayShortUrl(raw: string): string {
  return raw.replace(/^https?:\/\//, '')
}

export function plural(n: number, word: string): string {
  return `${n.toLocaleString('en')} ${word}${n === 1 ? '' : 's'}`
}

export function isExpired(expiresAt: string | null, now = Date.now()): boolean {
  return expiresAt !== null && new Date(expiresAt).getTime() <= now
}
