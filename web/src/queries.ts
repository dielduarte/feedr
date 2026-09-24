import {
  type InfiniteData,
  type QueryClient,
  useInfiniteQuery,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query'
import { useCallback, useEffect, useRef, useState } from 'react'
import { toast } from 'sonner'
import { api, type Item, type ItemSummary, type Page, type PollerEvent, type Sidebar } from './api'
import { scopePath, type Scope } from './routes'

export const keys = {
  sidebar: ['sidebar'] as const,
  items: (scope: Scope, unreadOnly: boolean) => ['items', scopePath(scope), unreadOnly] as const,
  item: (id: number) => ['item', id] as const,
}

export function useSidebar() {
  return useQuery({ queryKey: keys.sidebar, queryFn: api.sidebar })
}

export function useItems(scope: Scope, unreadOnly: boolean) {
  return useInfiniteQuery({
    queryKey: keys.items(scope, unreadOnly),
    queryFn: ({ pageParam }) => api.items(scope, unreadOnly, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.next_cursor,
  })
}

export function useItem(id: number | null) {
  return useQuery({
    queryKey: keys.item(id ?? 0),
    queryFn: () => api.item(id!),
    enabled: id !== null,
  })
}

type Snapshot = [readonly unknown[], unknown][]

function snapshot(client: QueryClient): Snapshot {
  return [
    ...client.getQueriesData({ queryKey: ['items'] }),
    ...client.getQueriesData({ queryKey: ['item'] }),
    ...client.getQueriesData({ queryKey: keys.sidebar }),
  ]
}

function restore(client: QueryClient, saved: Snapshot) {
  for (const [key, data] of saved) client.setQueryData(key, data)
}

function patchItem(client: QueryClient, id: number, patch: Partial<ItemSummary>) {
  client.setQueriesData<InfiniteData<Page>>({ queryKey: ['items'] }, (data) =>
    data && {
      ...data,
      pages: data.pages.map((page) => ({
        ...page,
        items: page.items.map((item) => (item.id === id ? { ...item, ...patch } : item)),
      })),
    },
  )
  client.setQueryData<Item>(keys.item(id), (item) => item && { ...item, ...patch })
}

function adjustUnread(client: QueryClient, feedId: number, delta: number) {
  client.setQueryData<Sidebar>(keys.sidebar, (sidebar) => {
    if (!sidebar) return sidebar
    const adjust = (feeds: Sidebar['uncategorized']) =>
      feeds.map((feed) => (feed.id === feedId ? { ...feed, unread: Math.max(0, feed.unread + delta) } : feed))
    return {
      total_unread: Math.max(0, sidebar.total_unread + delta),
      uncategorized: adjust(sidebar.uncategorized),
      folders: sidebar.folders.map((folder) =>
        folder.feeds.some((feed) => feed.id === feedId)
          ? { ...folder, unread: Math.max(0, folder.unread + delta), feeds: adjust(folder.feeds) }
          : folder,
      ),
    }
  })
}

/** Read and star changes apply everywhere at once, and roll back if the server refuses. */
export function useUpdateItem() {
  const client = useQueryClient()
  return useMutation({
    mutationFn: ({ item, patch }: { item: ItemSummary; patch: { read?: boolean; starred?: boolean } }) =>
      api.updateItem(item.id, patch),
    onMutate: async ({ item, patch }) => {
      await client.cancelQueries({ queryKey: ['items'] })
      const saved = snapshot(client)
      const now = new Date().toISOString()
      if (patch.read !== undefined && patch.read !== (item.read_at !== null)) {
        patchItem(client, item.id, { read_at: patch.read ? now : null })
        adjustUnread(client, item.feed_id, patch.read ? -1 : 1)
      }
      if (patch.starred !== undefined) {
        patchItem(client, item.id, { starred_at: patch.starred ? now : null })
      }
      return saved
    },
    onError: (error, _variables, saved) => {
      if (saved) restore(client, saved)
      toast.error(sentence(error.message))
    },
    onSettled: () => client.invalidateQueries({ queryKey: keys.sidebar }),
  })
}

export function useMarkAllRead() {
  const client = useQueryClient()
  return useMutation({
    mutationFn: ({ scope, upTo }: { scope: Scope; upTo: number }) => api.markRead(scope, upTo),
    onSuccess: () => {
      client.invalidateQueries({ queryKey: keys.sidebar })
      client.invalidateQueries({ queryKey: ['items'] })
      client.invalidateQueries({ queryKey: ['item'] })
    },
  })
}

/**
 * For sidebar edits: subscribe, move, rename, delete. Failures show as a toast unless the caller
 * displays them itself.
 */
export function useSidebarMutation<Args, Result>(run: (args: Args) => Promise<Result>, { inlineErrors = false } = {}) {
  const client = useQueryClient()
  return useMutation({
    mutationFn: run,
    onError: (error) => !inlineErrors && toast.error(sentence(error.message)),
    onSuccess: () => {
      client.invalidateQueries({ queryKey: keys.sidebar })
      client.invalidateQueries({ queryKey: ['items'] })
    },
  })
}

export type PollerStatus = { refreshing: boolean; offline: boolean }

/** How long a requested refresh may wait for the server to start before we stop showing it. */
const REFRESH_START_TIMEOUT = 10_000
/** One full turn of the spin animation, so even an instant refresh is visibly acknowledged. */
const MIN_REFRESH_DISPLAY = 1_000

/**
 * Follows the server's poller over SSE, refreshes what's on screen as feeds update, and
 * reports whether a refresh is in progress, including one just requested from here.
 */
export function usePoller() {
  const client = useQueryClient()
  const [status, setStatus] = useState<PollerStatus>({ refreshing: false, offline: false })
  const startTimeout = useRef<ReturnType<typeof setTimeout>>(undefined)
  const stopTimeout = useRef<ReturnType<typeof setTimeout>>(undefined)
  const shownSince = useRef(0)

  const showRefreshing = useCallback(() => {
    clearTimeout(stopTimeout.current)
    setStatus((s) => {
      if (!s.refreshing) shownSince.current = performance.now()
      return { ...s, refreshing: true }
    })
  }, [])

  const hideRefreshing = useCallback((offline?: boolean) => {
    const remaining = MIN_REFRESH_DISPLAY - (performance.now() - shownSince.current)
    clearTimeout(stopTimeout.current)
    stopTimeout.current = setTimeout(
      () => setStatus((s) => ({ refreshing: false, offline: offline ?? s.offline })),
      Math.max(0, remaining),
    )
  }, [])

  useEffect(() => {
    const source = new EventSource('/api/events')
    source.onmessage = (message) => {
      const event: PollerEvent = JSON.parse(message.data)
      switch (event.type) {
        case 'batch_started':
          clearTimeout(startTimeout.current)
          showRefreshing()
          break
        case 'feed_refreshed':
        case 'feed_failed':
          client.invalidateQueries({ queryKey: keys.sidebar })
          break
        case 'batch_finished':
          hideRefreshing(event.health === 'offline')
          client.invalidateQueries({ queryKey: keys.sidebar })
          client.invalidateQueries({ queryKey: ['items'] })
          break
        case 'resync':
          client.invalidateQueries()
          break
      }
    }
    return () => {
      source.close()
      clearTimeout(startTimeout.current)
      clearTimeout(stopTimeout.current)
    }
  }, [client, showRefreshing, hideRefreshing])

  // Shows progress from the click, before the server's batch begins; stops if nothing was due
  // or the server never picks it up.
  const refresh = useCallback(async (scope: Scope) => {
    const stop = () => hideRefreshing()
    showRefreshing()
    clearTimeout(startTimeout.current)
    startTimeout.current = setTimeout(stop, REFRESH_START_TIMEOUT)
    try {
      const { scheduled } = await api.refresh(scope)
      if (scheduled === 0) {
        clearTimeout(startTimeout.current)
        stop()
      }
    } catch (error) {
      clearTimeout(startTimeout.current)
      stop()
      toast.error(sentence((error as Error).message))
    }
  }, [showRefreshing, hideRefreshing])

  return { status, refresh }
}

function sentence(text: string) {
  return text.charAt(0).toUpperCase() + text.slice(1)
}
