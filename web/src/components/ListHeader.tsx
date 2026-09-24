import { ArrowUpRight } from 'lucide-react'
import type { ReactNode } from 'react'
import type { Sidebar } from '../api'
import type { Lookup } from '../lookup'
import type { Scope } from '../routes'

type Props = {
  heading: string
  scope: Scope
  sidebar: Sidebar | undefined
  lookup: Lookup
}

export function ListHeader({ heading, scope, sidebar, lookup }: Props) {
  const feed = scope.kind === 'feed' ? lookup.feed(scope.id) : undefined
  const details: ReactNode[] = []

  if (sidebar && (scope.kind === 'all' || scope.kind === 'unread')) details.push(`${sidebar.total_unread} unread`)
  if (scope.kind === 'folder') {
    const folder = lookup.folder(scope.id)
    if (folder) details.push(`${folder.unread} unread in ${folder.feeds.length} ${folder.feeds.length === 1 ? 'feed' : 'feeds'}`)
  }
  if (feed) {
    details.push(`${feed.unread} unread`)
    if (feed.site_url) {
      details.push(
        <a href={feed.site_url} target="_blank" rel="noopener noreferrer" className="inline-flex items-center gap-0.5 hover:text-foreground">
          {new URL(feed.site_url).hostname}
          <ArrowUpRight className="size-3.5" />
        </a>,
      )
    }
  }

  return (
    <header className="mb-5">
      <h1 className="text-3xl leading-tight font-semibold tracking-[-0.022em]">{heading}</h1>
      {details.length > 0 ? (
        <p className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-[13px] text-muted-foreground">
          {details.map((detail, i) => (
            <span key={i}>{detail}</span>
          ))}
        </p>
      ) : null}
      {feed?.last_error ? (
        <p className="mt-3.5 rounded-lg bg-warning-soft px-3 py-2 text-[13px] leading-normal text-warning">
          This feed isn't updating: {feed.last_error}. feedrsauros keeps retrying on its own.
        </p>
      ) : null}
    </header>
  )
}
