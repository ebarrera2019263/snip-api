// Typed client for snip-api. Mirrors the response shapes in src/models.rs.

export type User = {
  id: number
  email: string
  created_at: string
}

export type AuthResponse = {
  token: string
  user: User
}

export type Link = {
  id: number
  code: string
  short_url: string
  target_url: string
  is_active: boolean
  expires_at: string | null
  click_count: number
  created_at: string
  updated_at: string
}

export type LinkPage = {
  items: Link[]
  page: number
  per_page: number
  total: number
}

export type LinkStats = {
  link_id: number
  total_clicks: number
  last_clicked_at: string | null
  daily: { date: string; clicks: number }[]
  top_referrers: { referrer: string | null; clicks: number }[]
}

export type CreateLinkInput = {
  target_url: string
  code?: string
  expires_at?: string
}

export type UpdateLinkInput = {
  target_url?: string
  is_active?: boolean
}

/** Error returned by the API: `{ error: { code, message } }`. */
export class ApiError extends Error {
  status: number
  code: string

  constructor(status: number, code: string, message: string) {
    super(message)
    this.status = status
    this.code = code
  }
}

const TOKEN_KEY = 'snip.token'

export const tokenStore = {
  get: () => localStorage.getItem(TOKEN_KEY),
  set: (token: string) => localStorage.setItem(TOKEN_KEY, token),
  clear: () => localStorage.removeItem(TOKEN_KEY),
}

/** Called whenever the API rejects the token, so the app can sign out. */
let onUnauthorized: () => void = () => {}
export function setUnauthorizedHandler(handler: () => void) {
  onUnauthorized = handler
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const headers: Record<string, string> = {}
  const token = tokenStore.get()
  if (token) headers.Authorization = `Bearer ${token}`
  if (body !== undefined) headers['Content-Type'] = 'application/json'

  let res: Response
  try {
    res = await fetch(path, {
      method,
      headers,
      body: body === undefined ? undefined : JSON.stringify(body),
    })
  } catch {
    throw new ApiError(0, 'network', "Can't reach the server. Check that the API is running.")
  }

  if (res.status === 204) return undefined as T

  const data = await res.json().catch(() => null)
  if (!res.ok) {
    if (res.status === 401 && token) onUnauthorized()
    throw new ApiError(
      res.status,
      data?.error?.code ?? 'unknown',
      data?.error?.message ?? `Request failed with status ${res.status}`,
    )
  }
  return data as T
}

export const api = {
  register: (email: string, password: string) =>
    request<AuthResponse>('POST', '/api/auth/register', { email, password }),
  login: (email: string, password: string) =>
    request<AuthResponse>('POST', '/api/auth/login', { email, password }),
  me: () => request<User>('GET', '/api/auth/me'),

  listLinks: (page: number, perPage = 20) =>
    request<LinkPage>('GET', `/api/links?page=${page}&per_page=${perPage}`),
  getLink: (id: number) => request<Link>('GET', `/api/links/${id}`),
  createLink: (input: CreateLinkInput) => request<Link>('POST', '/api/links', input),
  updateLink: (id: number, input: UpdateLinkInput) =>
    request<Link>('PATCH', `/api/links/${id}`, input),
  deleteLink: (id: number) => request<void>('DELETE', `/api/links/${id}`),
  linkStats: (id: number) => request<LinkStats>('GET', `/api/links/${id}/stats`),
}
