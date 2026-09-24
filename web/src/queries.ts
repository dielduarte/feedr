import {
  type InfiniteData,
  type QueryClient,
  useInfiniteQuery,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query'
import { useMemo } from 'react'
import { toast } from 'sonner'
import { api, type Item, type ItemSummary, type Page, type Sidebar } from './api'
import { sentence } from './format'
import { buildLookup } from './lookup'
import { scopePath, type Scope } from './routes'

export const keys = {
  sidebar: ['sidebar'] as const,
  allItems: ['items'] as const,
  items: (scope: Scope, unreadOnly: boolean) => ['items', scopePath(scope), unreadOnly] as const,
  allItem: ['item'] as const,
  item: (id: number) => ['item', id] as const,
}

export function useSidebarData() {
  return useQuery({ queryKey: keys.sidebar, queryFn: api.sidebar })
}

export function useLookup(sidebar: Sidebar | undefined) {
  return useMemo(() => buildLookup(sidebar), [sidebar])
}

export function useItems(scope: Scope, unreadOnly: boolean) {
  const query = useInfiniteQuery({
    queryKey: keys.items(scope, unreadOnly),
    queryFn: ({ pageParam }) => api.items(scope, unreadOnly, pageParam),
    initialPageParam: null as string | null,
    getNextPageParam: (page) => page.next_cursor,
  })
  const items = useMemo(() => query.data?.pages.flatMap((page) => page.items) ?? [], [query.data])
  return { ...query, items }
}

export function useItem(id: number) {
  return useQuery({ queryKey: keys.item(id), queryFn: () => api.item(id) })
}

type Snapshot = [readonly unknown[], unknown][]

function snapshot(client: QueryClient): Snapshot {
  return [
    ...client.getQueriesData({ queryKey: keys.allItems }),
    ...client.getQueriesData({ queryKey: keys.allItem }),
    ...client.getQueriesData({ queryKey: keys.sidebar }),
  ]
}

function restore(client: QueryClient, saved: Snapshot) {
  for (const [key, data] of saved) client.setQueryData(key, data)
}

function patchItem(client: QueryClient, id: number, patch: Partial<ItemSummary>) {
  client.setQueriesData<InfiniteData<Page>>({ queryKey: keys.allItems }, (data) =>
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

export type ItemPatch = { read?: boolean; starred?: boolean }

/** Read and star changes apply everywhere at once, and roll back if the server refuses. */
export function useUpdateItem() {
  const client = useQueryClient()
  return useMutation({
    mutationFn: ({ item, patch }: { item: ItemSummary; patch: ItemPatch }) => api.updateItem(item.id, patch),
    onMutate: async ({ item, patch }) => {
      await client.cancelQueries({ queryKey: keys.allItems })
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
  }).mutate
}

export function useMarkAllRead() {
  const client = useQueryClient()
  return useMutation({
    mutationFn: ({ scope, upTo }: { scope: Scope; upTo: number }) => api.markRead(scope, upTo),
    onSuccess: () => {
      client.invalidateQueries({ queryKey: keys.sidebar })
      client.invalidateQueries({ queryKey: keys.allItems })
      client.invalidateQueries({ queryKey: keys.allItem })
    },
    onError: (error) => toast.error(sentence(error.message)),
  }).mutate
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
      client.invalidateQueries({ queryKey: keys.allItems })
    },
  })
}

/** Every sidebar edit, as stable functions safe to pass to memoized components. */
export function useSubscriptionActions() {
  const moveFeed = useSidebarMutation((a: { id: number; folderId: number | null; index: number }) =>
    api.moveFeed(a.id, a.folderId, a.index),
  ).mutate
  const moveFolder = useSidebarMutation((a: { id: number; index: number }) => api.moveFolder(a.id, a.index)).mutate
  const renameFeed = useSidebarMutation((a: { id: number; title: string | null }) => api.renameFeed(a.id, a.title)).mutate
  const renameFolder = useSidebarMutation((a: { id: number; name: string }) => api.renameFolder(a.id, a.name)).mutate
  const createFolder = useSidebarMutation((name: string) => api.createFolder(name)).mutate
  const unsubscribe = useSidebarMutation((id: number) => api.unsubscribe(id)).mutate
  const deleteFolder = useSidebarMutation((id: number) => api.deleteFolder(id)).mutate
  return useMemo(
    () => ({ moveFeed, moveFolder, renameFeed, renameFolder, createFolder, unsubscribe, deleteFolder }),
    [moveFeed, moveFolder, renameFeed, renameFolder, createFolder, unsubscribe, deleteFolder],
  )
}
