import type { Sidebar, SidebarFeed, SidebarFolder } from './api'
import type { Scope } from './routes'

export type Lookup = {
  feed: (slug: string) => SidebarFeed | undefined
  folder: (slug: string) => SidebarFolder | undefined
  hasFeeds: boolean
}

/** Indexes the sidebar once so feeds and folders can be found by slug without rescanning it. */
export function buildLookup(sidebar: Sidebar | undefined): Lookup {
  const feeds = new Map<string, SidebarFeed>()
  const folders = new Map<string, SidebarFolder>()
  for (const folder of sidebar?.folders ?? []) {
    folders.set(folder.slug, folder)
    for (const feed of folder.feeds) feeds.set(feed.slug, feed)
  }
  for (const feed of sidebar?.uncategorized ?? []) feeds.set(feed.slug, feed)

  return {
    feed: (slug) => feeds.get(slug),
    folder: (slug) => folders.get(slug),
    hasFeeds: feeds.size > 0,
  }
}

export function scopeLabel(scope: Scope, lookup: Lookup): string {
  switch (scope.kind) {
    case 'all':
      return 'All articles'
    case 'unread':
      return 'Unread'
    case 'starred':
      return 'Starred'
    case 'folder':
      return lookup.folder(scope.slug)?.name ?? 'Folder'
    case 'feed':
      return lookup.feed(scope.slug)?.title ?? 'Feed'
  }
}
