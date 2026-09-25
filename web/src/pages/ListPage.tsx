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
import { refOf, useItems, useMarkAllRead, useUpdateItem } from '../queries'
import { type ArticleRef, articleKey } from '../routes'

/** Keep this many articles of headroom: reaching them loads the next page. */
const PREFETCH_ROWS = 5

type Props = {
  chrome: Chrome
  unreadPreference: boolean
  onUnreadPreferenceChange: (unreadOnly: boolean) => void
  /** `feed/slug` of the article last opened, so returning from it keeps your place. */
  initialSelectedKey: string | null
  onOpen: (article: ArticleRef) => void
  onAddFeed: () => void
  onImport: () => void
}

export function ListPage({ chrome, unreadPreference, onUnreadPreferenceChange, initialSelectedKey, onOpen, onAddFeed, onImport }: Props) {
  const { scope, sidebar, lookup } = chrome
  const unreadOnly = listsUnreadOnly(scope, unreadPreference)
  const { items, isPending, hasNextPage, isFetchingNextPage, fetchNextPage } = useItems(scope, unreadOnly)
  const [selectedKey, setSelectedKey] = useState(initialSelectedKey)
  const updateItem = useUpdateItem()
  const markAllRead = useMarkAllRead()

  // One pass for everything keyboard navigation and "mark all read" need.
  const { keys, byKey, seenUntil } = useMemo(() => {
    const keys: string[] = []
    const byKey = new Map<string, ItemSummary>()
    let seenUntil: string | null = null
    for (const item of items) {
      const key = articleKey(refOf(item))
      keys.push(key)
      byKey.set(key, item)
      if (seenUntil === null || item.fetched_at > seenUntil) seenUntil = item.fetched_at
    }
    return { keys, byKey, seenUntil }
  }, [items])
  const selected = selectedKey === null ? undefined : byKey.get(selectedKey)

  const loadMore = useCallback(() => {
    if (hasNextPage && !isFetchingNextPage) fetchNextPage()
  }, [hasNextPage, isFetchingNextPage, fetchNextPage])

  const toggleStar = useCallback(
    (item: ItemSummary) => updateItem({ item, patch: { starred: !item.starred_at } }),
    [updateItem],
  )

  const move = (step: number) => {
    const next = moveSelection(keys, selectedKey, step)
    if (next === null) return
    setSelectedKey(next)
    if (nearEnd(keys, next, PREFETCH_ROWS)) loadMore()
  }
  const markAll = () => {
    if (seenUntil !== null) markAllRead({ scope, seenUntil })
  }

  useShortcuts(
    {
      ...chrome.shortcuts,
      j: () => move(1),
      k: () => move(-1),
      ArrowDown: () => move(1),
      ArrowUp: () => move(-1),
      Enter: () => selected && onOpen(refOf(selected)),
      o: () => selected && onOpen(refOf(selected)),
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
          canMarkAllRead={seenUntil !== null && scope.kind !== 'starred'}
          onMarkAllRead={markAll}
        />
      </TopBar>
      <div className="flex-1 overflow-y-auto">
        <div className="mx-auto max-w-190 px-8 pt-11 pb-24 max-md:px-5">
          <ListHeader heading={chrome.label} scope={scope} sidebar={sidebar} lookup={lookup} />
          {isPending ? null : items.length > 0 ? (
            <ArticleList
              items={items}
              selectedKey={selectedKey}
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
