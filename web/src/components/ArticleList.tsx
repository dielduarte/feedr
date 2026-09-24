import { Star } from 'lucide-react'
import { type ReactNode, useEffect, useRef } from 'react'
import { Link } from 'wouter'
import { Button } from '@/components/ui/button'
import { useNow } from '@/lib/hooks'
import { cn } from '@/lib/utils'
import type { ItemSummary } from '../api'
import { relativeTime } from '../format'
import { itemPath, type Scope } from '../routes'

type Props = {
  scope: Scope
  heading: string
  details: ReactNode
  items: ItemSummary[]
  loading: boolean
  hasMore: boolean
  onLoadMore: () => void
  selected: number
  onSelect: (index: number) => void
  onToggleStar: (item: ItemSummary) => void
  empty: ReactNode
}

export function ArticleList({ scope, heading, details, items, loading, hasMore, onLoadMore, selected, onSelect, onToggleStar, empty }: Props) {
  const now = useNow()
  const list = useRef<HTMLOListElement>(null)
  const sentinel = useRef<HTMLDivElement>(null)

  useEffect(() => {
    list.current?.querySelector<HTMLElement>(`[data-index="${selected}"]`)?.scrollIntoView({ block: 'nearest' })
  }, [selected])

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
    <div className="mx-auto max-w-190 px-8 pt-11 pb-24 max-md:px-5">
      <header className="mb-5">
        <h1 className="text-3xl leading-tight font-semibold tracking-[-0.022em]">{heading}</h1>
        {details}
      </header>

      {!loading && items.length === 0 ? (
        <div className="py-14">{empty}</div>
      ) : (
        <ol ref={list} className="-mx-3">
          {items.map((item, index) => {
            const unread = item.read_at === null
            return (
              <li
                key={item.id}
                data-index={index}
                className="group/article relative"
                onMouseMove={() => index !== selected && onSelect(index)}
              >
                <Link
                  href={itemPath(scope, item.id)}
                  className={cn(
                    'flex rounded-[10px] py-3 pr-12 pl-3 focus-visible:-outline-offset-2',
                    index === selected && 'bg-secondary',
                  )}
                >
                  <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                    <span className="flex items-baseline gap-3">
                      <span className={cn('flex-1 text-[15px] leading-snug font-medium', unread ? 'text-foreground' : 'text-muted-foreground')}>
                        {unread && <span className="sr-only">Unread: </span>}
                        {item.title ?? 'Untitled'}
                      </span>
                      <time dateTime={item.published_at} className="text-xs whitespace-nowrap text-faint tabular-nums">
                        {relativeTime(new Date(item.published_at), now)}
                      </time>
                    </span>
                    <span className={cn('line-clamp-2 text-[13px] leading-normal', unread ? 'text-muted-foreground' : 'text-faint')}>
                      <span className={cn('mr-2 font-medium', unread ? 'text-foreground' : 'text-muted-foreground')}>{item.feed_title}</span>
                      {item.summary}
                    </span>
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
                    item.starred_at ? 'text-foreground [&_svg]:fill-current' : 'opacity-0 group-hover/article:opacity-100 focus-visible:opacity-100',
                    index === selected && 'opacity-100',
                  )}
                >
                  <Star className="size-[15px]" />
                </Button>
              </li>
            )
          })}
        </ol>
      )}
      {hasMore && <div ref={sentinel} className="h-px" aria-hidden />}
    </div>
  )
}
