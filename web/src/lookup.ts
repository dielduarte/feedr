import type { Sidebar, SidebarFeed, SidebarFolder } from './api'
import type { Scope } from './routes'

export type Lookup = {
  feed: (id: number) => SidebarFeed | undefined
  folder: (id: number) => SidebarFolder | undefined
  hasFeeds: boolean
}

/** Indexes the sidebar once so feeds and folders can be found by id without rescanning it. */
export function buildLookup(sidebar: Sidebar | undefined): Lookup {
  const feeds = new Map<number, SidebarFeed>()
  const folders = new Map<number, SidebarFolder>()
  for (const folder of sidebar?.folders ?? []) {
    folders.set(folder.id, folder)
    for (const feed of folder.feeds) feeds.set(feed.id, feed)
  }
  for (const feed of sidebar?.uncategorized ?? []) feeds.set(feed.id, feed)

  return {
    feed: (id) => feeds.get(id),
    folder: (id) => folders.get(id),
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
      return lookup.folder(scope.id)?.name ?? 'Folder'
    case 'feed':
      return lookup.feed(scope.id)?.title ?? 'Feed'
  }
}
