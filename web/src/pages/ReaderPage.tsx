import { useEffect, useMemo, useRef } from 'react'
import { useDocumentTitle } from '@/hooks/use-document-title'
import { useShortcuts } from '@/hooks/use-shortcuts'
import type { Chrome } from '../chrome'
import { Reader } from '../components/Reader'
import { ReaderActions, TopBar } from '../components/TopBar'
import { wordCount } from '../format'
import { nearEnd, neighbour } from '../navigation'
import { refOf, useItem, useItems, useUpdateItem } from '../queries'
import { type ArticleRef, articleKey } from '../routes'

const PREFETCH_ROWS = 5

type Props = {
  chrome: Chrome
  article: ArticleRef
  unreadOnly: boolean
  onBack: () => void
  onOpen: (article: ArticleRef) => void
}

export function ReaderPage({ chrome, article, unreadOnly, onBack, onOpen }: Props) {
  const current = articleKey(article)
  const { data: item, isError } = useItem(article)
  // The list this article was opened from, for moving to the next and previous one.
  const { items, hasNextPage, isFetchingNextPage, fetchNextPage } = useItems(chrome.scope, unreadOnly)
  const { keys, byKey } = useMemo(() => {
    const byKey = new Map<string, ArticleRef>()
    for (const listed of items) byKey.set(articleKey(refOf(listed)), refOf(listed))
    return { keys: [...byKey.keys()], byKey }
  }, [items])
  const updateItem = useUpdateItem()
  const scroller = useRef<HTMLDivElement>(null)
  const html = item?.content_html
  const words = useMemo(() => (html ? wordCount(html) : 0), [html])

  // Opening an article marks it read once, whether it was opened from the list or a link;
  // pressing M afterwards is respected.
  const markedOnOpen = useRef<string | null>(null)
  useEffect(() => {
    if (item && !item.read_at && markedOnOpen.current !== articleKey(refOf(item))) {
      markedOnOpen.current = articleKey(refOf(item))
      updateItem({ item, patch: { read: true } })
    }
  }, [item, updateItem])

  useEffect(() => {
    scroller.current?.scrollTo({ top: 0 })
  }, [current])

  useDocumentTitle(item?.title ?? chrome.label, chrome.sidebar?.total_unread ?? 0)

  const step = (delta: number) => {
    const next = neighbour(keys, current, delta)
    const nextArticle = next === null ? undefined : byKey.get(next)
    if (nextArticle) onOpen(nextArticle)
    if (nearEnd(keys, next ?? current, PREFETCH_ROWS) && hasNextPage && !isFetchingNextPage) fetchNextPage()
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
          siteUrl={item ? (chrome.lookup.feed(item.feed_slug)?.site_url ?? null) : null}
          missing={isError}
        />
      </div>
    </>
  )
}
