import { ArrowUpRight } from 'lucide-react'
import { useEffect, useRef } from 'react'
import { Link } from 'wouter'
import type { Item } from '../api'
import { fullDate, readingMinutes } from '../format'
import { scopePath } from '../routes'
import { DinoLoader } from './DinoLoader'
import { FeedIcon } from './FeedIcon'

type Props = {
  item: Item | undefined
  words: number
  siteUrl: string | null
  missing: boolean
}

const prose = [
  'prose prose-neutral max-w-none text-[#2a2a2d] prose-p:leading-[1.72]',
  'prose-headings:font-semibold prose-headings:tracking-tight prose-headings:text-foreground prose-h1:text-[21px] prose-h2:text-[21px] prose-h3:text-[17px]',
  'prose-a:font-normal prose-a:text-foreground prose-a:decoration-signal/70 prose-a:decoration-[1.5px] prose-a:underline-offset-3 hover:prose-a:decoration-foreground',
  'prose-img:rounded-[10px] prose-video:rounded-[10px] prose-figcaption:text-muted-foreground',
  'prose-blockquote:border-l-2 prose-blockquote:font-normal prose-blockquote:not-italic prose-blockquote:text-muted-foreground',
  'prose-code:rounded-[5px] prose-code:bg-secondary prose-code:px-[0.35em] prose-code:py-[0.1em] prose-code:font-normal prose-code:before:content-none prose-code:after:content-none',
  'prose-pre:rounded-[10px] prose-pre:bg-[#f6f6f7] prose-pre:text-foreground [&_pre_code]:bg-transparent [&_pre_code]:p-0',
].join(' ')

export function Reader({ item, words, siteUrl, missing }: Props) {
  const content = useRef<HTMLDivElement>(null)

  // Links inside articles lead away from feedrsauros, so they get their own tab.
  useEffect(() => {
    content.current?.querySelectorAll('a[href]').forEach((link) => {
      link.setAttribute('target', '_blank')
      link.setAttribute('rel', 'noopener noreferrer')
    })
  }, [item?.content_html])

  if (missing) {
    return (
      <div className="mx-auto max-w-170 px-8 py-18">
        <p className="text-[17px] font-medium">This article no longer exists.</p>
      </div>
    )
  }

  if (!item) {
    return <DinoLoader label="Loading the article…" />
  }

  return (
    <article className="mx-auto max-w-170 px-8 pt-18 pb-30 max-md:px-5 max-md:pt-10">
      <h1 className="mb-4 text-4xl leading-[1.12] font-semibold tracking-[-0.028em] text-balance max-md:text-[28px]">
        {item.title ?? 'Untitled'}
      </h1>
      <div className="mb-9 flex flex-wrap items-center gap-x-4.5 gap-y-1.5 text-[13.5px] text-muted-foreground">
        <Link
          href={scopePath({ kind: 'feed', slug: item.feed_slug })}
          className="inline-flex items-center gap-2 font-medium text-foreground hover:underline hover:underline-offset-3"
        >
          <FeedIcon siteUrl={siteUrl} />
          {item.feed_title}
        </Link>
        {item.author && <span>{item.author}</span>}
        <time dateTime={item.published_at}>{fullDate(new Date(item.published_at))}</time>
        {words > 0 && <span>{readingMinutes(words)} min read</span>}
      </div>

      {item.content_html ? (
        <div ref={content} className={prose} dangerouslySetInnerHTML={{ __html: item.content_html }} />
      ) : (
        <p className="text-muted-foreground">This feed only publishes a link for this article.</p>
      )}

      {item.url && (
        <a
          href={item.url}
          target="_blank"
          rel="noopener noreferrer"
          className="mt-10 inline-flex items-center gap-1 border-b pb-0.5 text-muted-foreground hover:border-muted-foreground hover:text-foreground"
        >
          Read on {new URL(item.url).hostname}
          <ArrowUpRight className="size-[15px]" />
        </a>
      )}
    </article>
  )
}
