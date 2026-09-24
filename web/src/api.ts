import type { Scope } from './routes'

export interface SidebarFeed {
  id: number
  title: string
  url: string
  site_url: string | null
  unread: number
  last_error: string | null
}

export interface SidebarFolder {
  id: number
  name: string
  unread: number
  feeds: SidebarFeed[]
}

export interface Sidebar {
  total_unread: number
  folders: SidebarFolder[]
  uncategorized: SidebarFeed[]
}

export interface ItemSummary {
  id: number
  feed_id: number
  feed_title: string
  url: string | null
  title: string | null
  author: string | null
  summary: string | null
  published_at: string
  read_at: string | null
  starred_at: string | null
}

export interface Item extends ItemSummary {
  content_html: string | null
}

export interface Page {
  items: ItemSummary[]
  next_cursor: string | null
}

export interface Added {
  id: number
  title: string
  new_items: number
}

export interface ImportReport {
  added: number
  skipped: number
  invalid: string[]
}

export type PollerEvent =
  | { type: 'batch_started'; feeds: number }
  | { type: 'feed_refreshed'; feed: number; new_items: number }
  | { type: 'feed_failed'; feed: number; error: string }
  | { type: 'batch_finished'; health: 'online' | 'offline' }
  | { type: 'resync' }

export class ApiError extends Error {
  readonly status: number

  constructor(status: number, message: string) {
    super(message)
    this.status = status
  }
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const response = await fetch(path, {
    method,
    headers: body === undefined ? undefined : { 'content-type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  })
  if (!response.ok) {
    const error = await response.json().catch(() => null)
    throw new ApiError(response.status, error?.error ?? `Request failed (${response.status})`)
  }
  return response.status === 204 ? (undefined as T) : response.json()
}

type ScopeFilter = { feed?: number; folder?: number; starred?: boolean }

function scopeFilter(scope: Scope): ScopeFilter {
  switch (scope.kind) {
    case 'folder':
      return { folder: scope.id }
    case 'feed':
      return { feed: scope.id }
    case 'starred':
      return { starred: true }
    case 'all':
    case 'unread':
      return {}
  }
}

export function listsUnreadOnly(scope: Scope, unreadOnly: boolean): boolean {
  return scope.kind === 'unread' || (unreadOnly && scope.kind !== 'starred')
}

export const api = {
  sidebar: () => request<Sidebar>('GET', '/api/sidebar'),

  items(scope: Scope, unreadOnly: boolean, cursor: string | null) {
    const query = new URLSearchParams()
    for (const [key, value] of Object.entries(scopeFilter(scope))) query.set(key, String(value))
    if (listsUnreadOnly(scope, unreadOnly)) query.set('unread', 'true')
    if (cursor) query.set('cursor', cursor)
    return request<Page>('GET', `/api/items?${query}`)
  },

  item: (id: number) => request<Item>('GET', `/api/items/${id}`),
  updateItem: (id: number, patch: { read?: boolean; starred?: boolean }) =>
    request<void>('PATCH', `/api/items/${id}`, patch),
  markRead: (scope: Scope, upTo: number) =>
    request<{ marked: number }>('POST', '/api/items/mark-read', {
      ...scopeFilter(scope),
      up_to: upTo,
    }),

  subscribe: (url: string, folderId: number | null) =>
    request<Added>('POST', '/api/feeds', { url, folder_id: folderId }),
  unsubscribe: (id: number) => request<void>('DELETE', `/api/feeds/${id}`),
  moveFeed: (id: number, folderId: number | null, index: number) =>
    request<void>('PUT', `/api/feeds/${id}/position`, { folder_id: folderId, index }),
  renameFeed: (id: number, title: string | null) =>
    request<void>('PUT', `/api/feeds/${id}/title`, { title }),

  createFolder: (name: string) => request<{ id: number; name: string }>('POST', '/api/folders', { name }),
  renameFolder: (id: number, name: string) => request<void>('PATCH', `/api/folders/${id}`, { name }),
  moveFolder: (id: number, index: number) =>
    request<void>('PUT', `/api/folders/${id}/position`, { index }),
  deleteFolder: (id: number) => request<void>('DELETE', `/api/folders/${id}`),

  refresh(scope: Scope) {
    const body = scope.kind === 'feed' ? { feed: scope.id } : scope.kind === 'folder' ? { folder: scope.id } : {}
    return request<{ scheduled: number }>('POST', '/api/refresh', body)
  },

  async importOpml(file: File): Promise<ImportReport> {
    const response = await fetch('/api/opml', {
      method: 'POST',
      headers: { 'content-type': 'text/x-opml' },
      body: await file.text(),
    })
    const body = await response.json()
    if (!response.ok) throw new ApiError(response.status, body.error)
    return body
  },
}
