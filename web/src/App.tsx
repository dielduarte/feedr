import { ArrowUpRight } from 'lucide-react'
import { type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useLocation } from 'wouter'
import { Button } from '@/components/ui/button'
import { SidebarInset, SidebarProvider, useSidebar } from '@/components/ui/sidebar'
import { useShortcuts, useStoredState } from '@/lib/hooks'
import { listsUnreadOnly, type Sidebar as SidebarData, type SidebarFeed } from './api'
import { AppSidebar } from './components/AppSidebar'
import { ArticleList } from './components/ArticleList'
import { AddFeedDialog, ShortcutsDialog, TransferDialog } from './components/Dialogs'
import { Reader } from './components/Reader'
import { TopBar } from './components/TopBar'
import { wordCount } from './format'
import { useItem, useItems, useMarkAllRead, usePoller, useSidebar as useSidebarData, useUpdateItem } from './queries'
import { itemPath, parseLocation, scopePath, type Scope } from './routes'

type Dialog = 'add' | 'transfer' | 'shortcuts' | null

export function App() {
  const [open, setOpen] = useStoredState('feedrsauros.sidebarOpen', true)
  return (
    <SidebarProvider open={open} onOpenChange={setOpen}>
      <Shell />
    </SidebarProvider>
  )
}

function findFeed(sidebar: SidebarData | undefined, id: number): SidebarFeed | undefined {
  return [...(sidebar?.uncategorized ?? []), ...(sidebar?.folders.flatMap((f) => f.feeds) ?? [])].find(
    (feed) => feed.id === id,
  )
}

function scopeLabel(scope: Scope, sidebar: SidebarData | undefined): string {
  switch (scope.kind) {
    case 'all':
      return 'All articles'
    case 'unread':
      return 'Unread'
    case 'starred':
      return 'Starred'
    case 'folder':
      return sidebar?.folders.find((f) => f.id === scope.id)?.name ?? 'Folder'
    case 'feed':
      return findFeed(sidebar, scope.id)?.title ?? 'Feed'
  }
}

function Shell() {
  const [path, setPath] = useLocation()
  const { scope, itemId } = parseLocation(path)
  const { toggleSidebar, isMobile, setOpenMobile } = useSidebar()
  const [unreadPreference, setUnreadPreference] = useStoredState('feedrsauros.unreadOnly', false)
  const [dialog, setDialog] = useState<Dialog>(null)
  const [selectedId, setSelectedId] = useState<number | null>(null)

  const { status, refresh: refreshScope } = usePoller()
  const { data: sidebar } = useSidebarData()
  const unreadOnly = listsUnreadOnly(scope, unreadPreference)
  const itemsQuery = useItems(scope, unreadOnly)
  const items = useMemo(() => itemsQuery.data?.pages.flatMap((page) => page.items) ?? [], [itemsQuery.data])
  const itemQuery = useItem(itemId)
  const item = itemQuery.data?.id === itemId ? itemQuery.data : undefined
  const updateItem = useUpdateItem()
  const markAllRead = useMarkAllRead()

  const label = scopeLabel(scope, sidebar)
  const words = useMemo(() => wordCount(item?.content_html ?? ''), [item?.content_html])
  const selected = Math.max(0, items.findIndex((i) => i.id === selectedId))
  const selectedItem = items[selected]
  const openIndex = itemId === null ? -1 : items.findIndex((i) => i.id === itemId)
  const scroller = useRef<HTMLDivElement>(null)

  const navigate = useCallback(
    (target: Scope) => {
      setPath(scopePath(target))
      setSelectedId(null)
      if (isMobile) setOpenMobile(false)
    },
    [setPath, isMobile, setOpenMobile],
  )
  const open = (id: number) => {
    setSelectedId(id)
    setPath(itemPath(scope, id))
  }
  const back = () => setPath(scopePath(scope))

  // Opening an article marks it read once; pressing M afterwards is respected.
  const markedOnOpen = useRef<number | null>(null)
  useEffect(() => {
    if (item && !item.read_at && markedOnOpen.current !== item.id) {
      markedOnOpen.current = item.id
      updateItem.mutate({ item, patch: { read: true } })
    }
    if (item) setSelectedId(item.id)
  }, [item, updateItem])

  useEffect(() => {
    scroller.current?.scrollTo({ top: 0 })
  }, [itemId])

  useEffect(() => {
    const unread = sidebar?.total_unread ?? 0
    document.title = `${unread > 0 ? `(${unread}) ` : ''}${item?.title ?? label} — feedrsauros`
  }, [sidebar?.total_unread, item?.title, label])

  const { hasNextPage, isFetchingNextPage, fetchNextPage } = itemsQuery
  const loadMore = useCallback(() => {
    if (hasNextPage && !isFetchingNextPage) fetchNextPage()
  }, [hasNextPage, isFetchingNextPage, fetchNextPage])

  const refresh = () => refreshScope(scope.kind === 'feed' || scope.kind === 'folder' ? scope : { kind: 'all' })
  const current = item ?? selectedItem
  const toggleStar = (target = current) => target && updateItem.mutate({ item: target, patch: { starred: !target.starred_at } })
  const toggleRead = (target = current) => target && updateItem.mutate({ item: target, patch: { read: !target.read_at } })
  const openOriginal = () => current?.url && window.open(current.url, '_blank', 'noopener')
  const newestLoaded = items.reduce((max, i) => Math.max(max, i.id), 0)
  const markAll = () => newestLoaded > 0 && markAllRead.mutate({ scope, upTo: newestLoaded })
  const moveSelection = (step: number) => {
    const next = Math.min(items.length - 1, Math.max(0, selected + step))
    if (items[next]) setSelectedId(items[next].id)
    if (next >= items.length - 5) loadMore()
  }
  const step = (delta: number) => {
    const next = items[openIndex + delta]
    if (openIndex !== -1 && next) open(next.id)
    if (openIndex + delta >= items.length - 5) loadMore()
  }

  const common = {
    s: () => toggleStar(),
    m: () => toggleRead(),
    v: openOriginal,
    r: refresh,
    '[': toggleSidebar,
    '?': () => setDialog('shortcuts'),
  }
  useShortcuts(
    itemId === null
      ? {
          ...common,
          j: () => moveSelection(1),
          k: () => moveSelection(-1),
          ArrowDown: () => moveSelection(1),
          ArrowUp: () => moveSelection(-1),
          Enter: () => selectedItem && open(selectedItem.id),
          o: () => selectedItem && open(selectedItem.id),
          A: markAll,
        }
      : { ...common, j: () => step(1), k: () => step(-1), Escape: back, u: back },
    dialog === null,
  )

  const feed = scope.kind === 'feed' ? findFeed(sidebar, scope.id) : undefined
  const hasSubscriptions = !!sidebar && (sidebar.uncategorized.length > 0 || sidebar.folders.some((f) => f.feeds.length > 0))
  const closeDialog = (open: boolean) => !open && setDialog(null)

  return (
    <>
      <AppSidebar
        scope={scope}
        sidebar={sidebar}
        onNavigate={navigate}
        onAddFeed={() => setDialog('add')}
        onOpenTransfer={() => setDialog('transfer')}
        onOpenShortcuts={() => setDialog('shortcuts')}
      />

      <SidebarInset className="h-dvh min-w-0 overflow-hidden md:h-[calc(100dvh-1rem)] md:border md:shadow-[0_1px_2px_rgb(0_0_0/0.03),0_18px_40px_-20px_rgb(0_0_0/0.12)]">
        <TopBar
          scope={scope}
          scopeLabel={label}
          sidebar={sidebar}
          onNavigate={navigate}
          status={status}
          onRefresh={refresh}
          view={
            itemId === null
              ? {
                  kind: 'list',
                  actions: {
                    unreadOnly,
                    canFilterUnread: scope.kind !== 'unread' && scope.kind !== 'starred',
                    onUnreadOnlyChange: setUnreadPreference,
                    canMarkAllRead: newestLoaded > 0 && scope.kind !== 'starred',
                    onMarkAllRead: markAll,
                  },
                }
              : { kind: 'reader', actions: { item, words, onBack: back, onToggleStar: () => toggleStar(), onToggleRead: () => toggleRead() } }
          }
        />
        <div ref={scroller} className="flex-1 overflow-y-auto">
          {itemId !== null ? (
            <Reader item={item} words={words} siteUrl={item ? findFeed(sidebar, item.feed_id)?.site_url ?? null : null} missing={itemQuery.isError} />
          ) : (
            <ArticleList
              scope={scope}
              heading={label}
              details={<ListDetails scope={scope} sidebar={sidebar} feed={feed} />}
              items={items}
              loading={itemsQuery.isPending}
              hasMore={!!hasNextPage}
              onLoadMore={loadMore}
              selected={selected}
              onSelect={(index) => setSelectedId(items[index]?.id ?? null)}
              onToggleStar={(target) => toggleStar(target)}
              empty={
                sidebar && !hasSubscriptions ? (
                  <Welcome onAdd={() => setDialog('add')} onImport={() => setDialog('transfer')} />
                ) : (
                  <EmptyList scope={scope} unreadOnly={unreadOnly} />
                )
              }
            />
          )}
        </div>
      </SidebarInset>

      <AddFeedDialog
        open={dialog === 'add'}
        onOpenChange={closeDialog}
        sidebar={sidebar}
        defaultFolder={scope.kind === 'folder' ? scope.id : null}
        onAdded={(id) => {
          setDialog(null)
          navigate({ kind: 'feed', id })
        }}
      />
      <TransferDialog open={dialog === 'transfer'} onOpenChange={closeDialog} />
      <ShortcutsDialog open={dialog === 'shortcuts'} onOpenChange={closeDialog} />
    </>
  )
}

function ListDetails({ scope, sidebar, feed }: { scope: Scope; sidebar: SidebarData | undefined; feed: SidebarFeed | undefined }) {
  if (!sidebar) return null
  const parts: ReactNode[] = []
  if (scope.kind === 'all' || scope.kind === 'unread') parts.push(`${sidebar.total_unread} unread`)
  if (scope.kind === 'folder') {
    const folder = sidebar.folders.find((f) => f.id === scope.id)
    if (folder) parts.push(`${folder.unread} unread in ${folder.feeds.length} ${folder.feeds.length === 1 ? 'feed' : 'feeds'}`)
  }
  if (feed) {
    parts.push(`${feed.unread} unread`)
    if (feed.site_url) {
      parts.push(
        <a key="site" href={feed.site_url} target="_blank" rel="noopener noreferrer" className="inline-flex items-center gap-0.5 hover:text-foreground">
          {new URL(feed.site_url).hostname}
          <ArrowUpRight className="size-3.5" />
        </a>,
      )
    }
  }
  return (
    <>
      <p className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-[13px] text-muted-foreground">
        {parts.map((part, i) => (
          <span key={i}>{part}</span>
        ))}
      </p>
      {feed?.last_error && (
        <p className="mt-3.5 rounded-lg bg-warning-soft px-3 py-2 text-[13px] leading-normal text-warning">
          This feed isn't updating: {feed.last_error}. feedrsauros keeps retrying on its own.
        </p>
      )}
    </>
  )
}

function EmptyList({ scope, unreadOnly }: { scope: Scope; unreadOnly: boolean }) {
  const [title, text] =
    scope.kind === 'starred'
      ? ['Nothing starred yet', 'Press S on an article to keep it here.']
      : unreadOnly
        ? ["You're all caught up", 'New articles show up here as your feeds update.']
        : ['No articles yet', "feedrsauros checks your feeds regularly; articles appear as they're published."]
  return (
    <>
      <p className="mb-1.5 text-[17px] font-medium">{title}</p>
      <p className="max-w-[46ch] leading-relaxed text-muted-foreground">{text}</p>
    </>
  )
}

function Welcome({ onAdd, onImport }: { onAdd: () => void; onImport: () => void }) {
  return (
    <>
      <p className="mb-2 text-2xl font-semibold tracking-[-0.02em]">Start with a site you read</p>
      <p className="max-w-[46ch] leading-relaxed text-muted-foreground">
        Paste its address and feedrsauros finds the feed, then keeps it up to date while it runs.
      </p>
      <div className="mt-5.5 flex items-center gap-2">
        <Button onClick={onAdd}>Add a feed</Button>
        <Button variant="ghost" onClick={onImport}>Import from another reader</Button>
      </div>
    </>
  )
}
