import { useEffect, useMemo, useRef } from 'react'
import { useDocumentTitle } from '@/hooks/use-document-title'
import { useShortcuts } from '@/hooks/use-shortcuts'
import type { Chrome } from '../chrome'
import { Reader } from '../components/Reader'
import { ReaderActions, TopBar } from '../components/TopBar'
import { wordCount } from '../format'
import { nearEnd, neighbour } from '../navigation'
import { useItem, useItems, useUpdateItem } from '../queries'

const PREFETCH_ROWS = 5

type Props = {
  chrome: Chrome
  itemId: number
  unreadOnly: boolean
  onBack: () => void
  onOpen: (id: number) => void
}

export function ReaderPage({ chrome, itemId, unreadOnly, onBack, onOpen }: Props) {
  const { data: item, isError } = useItem(itemId)
  // The list this article was opened from, for moving to the next and previous one.
  const { items, hasNextPage, isFetchingNextPage, fetchNextPage } = useItems(chrome.scope, unreadOnly)
  const ids = useMemo(() => items.map((i) => i.id), [items])
  const updateItem = useUpdateItem()
  const scroller = useRef<HTMLDivElement>(null)
  const html = item?.content_html
  const words = useMemo(() => (html ? wordCount(html) : 0), [html])

  // Opening an article marks it read once, whether it was opened from the list or a link;
  // pressing M afterwards is respected.
  const markedOnOpen = useRef<number | null>(null)
  useEffect(() => {
    if (item && !item.read_at && markedOnOpen.current !== item.id) {
      markedOnOpen.current = item.id
      updateItem({ item, patch: { read: true } })
    }
  }, [item, updateItem])

  useEffect(() => {
    scroller.current?.scrollTo({ top: 0 })
  }, [itemId])

  useDocumentTitle(item?.title ?? chrome.label, chrome.sidebar?.total_unread ?? 0)

  const step = (delta: number) => {
    const next = neighbour(ids, itemId, delta)
    if (next !== null) onOpen(next)
    if (nearEnd(ids, next ?? itemId, PREFETCH_ROWS) && hasNextPage && !isFetchingNextPage) fetchNextPage()
  }
  const toggleStar = () => item && updateItem({ item, patch: { starred: !item.starred_at } })
  const toggleRead = () => item && updateItem({ item, patch: { read: !item.read_at } })

  useShortcuts(
    {
      ...chrome.shortcuts,
      j: () => step(1),
      k: () => step(-1),
      Escape: onBack,
      u: onBack,
      s: toggleStar,
      m: toggleRead,
      v: () => item?.url && window.open(item.url, '_blank', 'noopener'),
    },
    chrome.shortcutsEnabled,
  )

  return (
    <>
      <TopBar chrome={chrome} onBack={onBack} crumb={item?.title}>
        {item ? <ReaderActions item={item} words={words} onToggleStar={toggleStar} onToggleRead={toggleRead} /> : null}
      </TopBar>
      <div ref={scroller} className="flex-1 overflow-y-auto">
        <Reader
          item={item}
          words={words}
          siteUrl={item ? (chrome.lookup.feed(item.feed_id)?.site_url ?? null) : null}
          missing={isError}
        />
      </div>
    </>
  )
}
