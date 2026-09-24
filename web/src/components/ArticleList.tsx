import { Star } from 'lucide-react'
import { memo, useEffect, useRef } from 'react'
import { Link } from 'wouter'
import { Button } from '@/components/ui/button'
import { useNow } from '@/hooks/use-now'
import { cn } from '@/lib/utils'
import type { ItemSummary } from '../api'
import { fullDate, relativeTime } from '../format'
import { itemPath, type Scope } from '../routes'

type Props = {
  scope: Scope
  items: ItemSummary[]
  selectedId: number | null
  hasMore: boolean
  onLoadMore: () => void
  onToggleStar: (item: ItemSummary) => void
}

export function ArticleList({ scope, items, selectedId, hasMore, onLoadMore, onToggleStar }: Props) {
  const now = useNow()
  const list = useRef<HTMLOListElement>(null)
  const sentinel = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (selectedId === null) return
    list.current?.querySelector(`[data-id="${selectedId}"]`)?.scrollIntoView({ block: 'nearest' })
  }, [selectedId])

  useEffect(() => {
    const node = sentinel.current
    if (!node || !hasMore) return
    const observer = new IntersectionObserver((entries) => entries.some((e) => e.isIntersecting) && onLoadMore(), {
      rootMargin: '400px',
    })
    observer.observe(node)
    return () => observer.disconnect()
  }, [hasMore, onLoadMore])

  return (
    <>
      <ol ref={list} className="-mx-3">
        {items.map((item) => (
          <ArticleRow
            key={item.id}
            item={item}
            href={itemPath(scope, item.id)}
            selected={item.id === selectedId}
            now={now}
            onToggleStar={onToggleStar}
          />
        ))}
      </ol>
      {hasMore ? <div ref={sentinel} className="h-px" aria-hidden /> : null}
    </>
  )
}

type RowProps = {
  item: ItemSummary
  href: string
  selected: boolean
  now: Date
  onToggleStar: (item: ItemSummary) => void
}

/** Memoized so marking one article read or moving the selection re-renders only the rows involved. */
const ArticleRow = memo(function ArticleRow({ item, href, selected, now, onToggleStar }: RowProps) {
  const unread = item.read_at === null
  const published = new Date(item.published_at)

  return (
    <li
      data-id={item.id}
      // Rows far off-screen skip layout and paint until scrolled near.
      className="group/article relative [contain-intrinsic-size:auto_112px] [content-visibility:auto]"
    >
      <Link
        href={href}
        className={cn(
          'flex rounded-[10px] py-3 pr-12 pl-3 hover:bg-secondary focus-visible:-outline-offset-2',
          selected && 'bg-secondary',
        )}
      >
        <span className="flex min-w-0 flex-1 flex-col gap-1">
          <span className={cn('line-clamp-2 text-[15px] leading-snug font-medium', unread ? 'text-foreground' : 'text-muted-foreground')}>
            {unread ? <span className="sr-only">Unread: </span> : null}
            {item.title ?? 'Untitled'}
          </span>
          <time dateTime={item.published_at} title={fullDate(published)} className="text-[13px] text-faint tabular-nums">
            {relativeTime(published, now)}
          </time>
          {item.summary ? (
            <span className={cn('line-clamp-2 text-[13px] leading-normal', unread ? 'text-muted-foreground' : 'text-faint')}>
              {item.summary}
            </span>
          ) : null}
        </span>
      </Link>
      <Button
        variant="ghost"
        size="icon-xs"
        onClick={() => onToggleStar(item)}
        aria-label={item.starred_at ? 'Unstar' : 'Star'}
        aria-pressed={item.starred_at !== null}
        className={cn(
          'absolute top-2.5 right-2.5 size-7 text-faint hover:text-foreground',
          item.starred_at
            ? 'text-foreground [&_svg]:fill-current'
            : 'opacity-0 group-hover/article:opacity-100 focus-visible:opacity-100',
          selected && 'opacity-100',
        )}
      >
        <Star className="size-[15px]" />
      </Button>
    </li>
  )
})
