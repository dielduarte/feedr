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
import { type ArticleRef, articleKey, scopePath, type Scope } from './routes'

export const keys = {
  sidebar: ['sidebar'] as const,
  allItems: ['items'] as const,
  items: (scope: Scope, unreadOnly: boolean) => ['items', scopePath(scope), unreadOnly] as const,
  allItem: ['item'] as const,
  item: (article: ArticleRef) => ['item', articleKey(article)] as const,
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

export function useItem(article: ArticleRef) {
  return useQuery({ queryKey: keys.item(article), queryFn: () => api.item(article) })
}

/** Where an article from a list lives. */
export function refOf(item: ItemSummary): ArticleRef {
  return { feed: item.feed_slug, slug: item.slug }
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

function patchItem(client: QueryClient, article: ArticleRef, patch: Partial<ItemSummary>) {
  const key = articleKey(article)
  client.setQueriesData<InfiniteData<Page>>({ queryKey: keys.allItems }, (data) =>
    data && {
      ...data,
      pages: data.pages.map((page) => ({
        ...page,
        items: page.items.map((item) => (articleKey(refOf(item)) === key ? { ...item, ...patch } : item)),
      })),
    },
  )
  client.setQueryData<Item>(keys.item(article), (item) => item && { ...item, ...patch })
}

function adjustUnread(client: QueryClient, feedSlug: string, delta: number) {
  client.setQueryData<Sidebar>(keys.sidebar, (sidebar) => {
    if (!sidebar) return sidebar
    const adjust = (feeds: Sidebar['uncategorized']) =>
      feeds.map((feed) => (feed.slug === feedSlug ? { ...feed, unread: Math.max(0, feed.unread + delta) } : feed))
    return {
      total_unread: Math.max(0, sidebar.total_unread + delta),
      uncategorized: adjust(sidebar.uncategorized),
      folders: sidebar.folders.map((folder) =>
        folder.feeds.some((feed) => feed.slug === feedSlug)
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
    mutationFn: ({ item, patch }: { item: ItemSummary; patch: ItemPatch }) => api.updateItem(refOf(item), patch),
    onMutate: async ({ item, patch }) => {
      await client.cancelQueries({ queryKey: keys.allItems })
      const saved = snapshot(client)
      const now = new Date().toISOString()
      if (patch.read !== undefined && patch.read !== (item.read_at !== null)) {
        patchItem(client, refOf(item), { read_at: patch.read ? now : null })
        adjustUnread(client, item.feed_slug, patch.read ? -1 : 1)
      }
      if (patch.starred !== undefined) {
        patchItem(client, refOf(item), { starred_at: patch.starred ? now : null })
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
    mutationFn: ({ scope, seenUntil }: { scope: Scope; seenUntil: string }) => api.markRead(scope, seenUntil),
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
  const moveFeed = useSidebarMutation((a: { slug: string; folder: string | null; index: number }) =>
    api.moveFeed(a.slug, a.folder, a.index),
  ).mutate
  const moveFolder = useSidebarMutation((a: { slug: string; index: number }) => api.moveFolder(a.slug, a.index)).mutate
  const renameFeed = useSidebarMutation((a: { slug: string; title: string | null }) => api.renameFeed(a.slug, a.title)).mutate
  const renameFolder = useSidebarMutation((a: { slug: string; name: string }) => api.renameFolder(a.slug, a.name)).mutate
  const createFolder = useSidebarMutation((name: string) => api.createFolder(name)).mutate
  const unsubscribe = useSidebarMutation((slug: string) => api.unsubscribe(slug)).mutate
  const deleteFolder = useSidebarMutation((slug: string) => api.deleteFolder(slug)).mutate
  return useMemo(
    () => ({ moveFeed, moveFolder, renameFeed, renameFolder, createFolder, unsubscribe, deleteFolder }),
    [moveFeed, moveFolder, renameFeed, renameFolder, createFolder, unsubscribe, deleteFolder],
  )
}
