import { useCallback, useMemo, useState } from 'react'
import { useDocumentTitle } from '@/hooks/use-document-title'
import { useShortcuts } from '@/hooks/use-shortcuts'
import { type ItemSummary, listsUnreadOnly } from '../api'
import type { Chrome } from '../chrome'
import { ArticleList } from '../components/ArticleList'
import { EmptyList, Welcome } from '../components/EmptyStates'
import { ListHeader } from '../components/ListHeader'
import { ListActions, TopBar } from '../components/TopBar'
import { moveSelection, nearEnd } from '../navigation'
import { useItems, useMarkAllRead, useUpdateItem } from '../queries'

/** Keep this many articles of headroom: reaching them loads the next page. */
const PREFETCH_ROWS = 5

type Props = {
  chrome: Chrome
  unreadPreference: boolean
  onUnreadPreferenceChange: (unreadOnly: boolean) => void
  /** The article last opened, so returning from it keeps your place. */
  initialSelectedId: number | null
  onOpen: (id: number) => void
  onAddFeed: () => void
  onImport: () => void
}

export function ListPage({ chrome, unreadPreference, onUnreadPreferenceChange, initialSelectedId, onOpen, onAddFeed, onImport }: Props) {
  const { scope, sidebar, lookup } = chrome
  const unreadOnly = listsUnreadOnly(scope, unreadPreference)
  const { items, isPending, hasNextPage, isFetchingNextPage, fetchNextPage } = useItems(scope, unreadOnly)
  const [selectedId, setSelectedId] = useState(initialSelectedId)
  const updateItem = useUpdateItem()
  const markAllRead = useMarkAllRead()

  const { ids, byId, newest } = useMemo(() => {
    const ids: number[] = []
    const byId = new Map<number, ItemSummary>()
    let newest = 0
    for (const item of items) {
      ids.push(item.id)
      byId.set(item.id, item)
      if (item.id > newest) newest = item.id
    }
    return { ids, byId, newest }
  }, [items])
  const selected = selectedId === null ? undefined : byId.get(selectedId)

  const loadMore = useCallback(() => {
    if (hasNextPage && !isFetchingNextPage) fetchNextPage()
  }, [hasNextPage, isFetchingNextPage, fetchNextPage])

  const toggleStar = useCallback(
    (item: ItemSummary) => updateItem({ item, patch: { starred: !item.starred_at } }),
    [updateItem],
  )

  const move = (step: number) => {
    const next = moveSelection(ids, selectedId, step)
    if (next === null) return
    setSelectedId(next)
    if (nearEnd(ids, next, PREFETCH_ROWS)) loadMore()
  }
  const markAll = () => {
    if (newest > 0) markAllRead({ scope, upTo: newest })
  }

  useShortcuts(
    {
      ...chrome.shortcuts,
      j: () => move(1),
      k: () => move(-1),
      ArrowDown: () => move(1),
      ArrowUp: () => move(-1),
      Enter: () => selected && onOpen(selected.id),
      o: () => selected && onOpen(selected.id),
      s: () => selected && toggleStar(selected),
      m: () => selected && updateItem({ item: selected, patch: { read: !selected.read_at } }),
      v: () => selected?.url && window.open(selected.url, '_blank', 'noopener'),
      A: markAll,
    },
    chrome.shortcutsEnabled,
  )
  useDocumentTitle(chrome.label, sidebar?.total_unread ?? 0)

  return (
    <>
      <TopBar chrome={chrome}>
        <ListActions
          unreadOnly={unreadOnly}
          canFilterUnread={scope.kind !== 'unread' && scope.kind !== 'starred'}
          onUnreadOnlyChange={onUnreadPreferenceChange}
          canMarkAllRead={newest > 0 && scope.kind !== 'starred'}
          onMarkAllRead={markAll}
        />
      </TopBar>
      <div className="flex-1 overflow-y-auto">
        <div className="mx-auto max-w-190 px-8 pt-11 pb-24 max-md:px-5">
          <ListHeader heading={chrome.label} scope={scope} sidebar={sidebar} lookup={lookup} />
          {isPending ? null : items.length > 0 ? (
            <ArticleList
              scope={scope}
              items={items}
              selectedId={selectedId}
              hasMore={!!hasNextPage}
              onLoadMore={loadMore}
              onToggleStar={toggleStar}
            />
          ) : sidebar && !lookup.hasFeeds ? (
            <Welcome onAdd={onAddFeed} onImport={onImport} />
          ) : (
            <EmptyList scope={scope} unreadOnly={unreadOnly} />
          )}
        </div>
      </div>
    </>
  )
}
