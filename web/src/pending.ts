import type { Sidebar, SidebarFeed } from './api'

// Real slugs only contain a-z, 0-9 and "-", so this prefix can never collide with one.
const PREFIX = '~adding-'
let counter = 0

/** A temporary slug for a feed that's still being added. */
export function pendingSlug(): string {
  counter += 1
  return `${PREFIX}${counter}`
}

export function isPendingSlug(slug: string): boolean {
  return slug.startsWith(PREFIX)
}

/** The site's host, which names the feed until its real title is known. */
export function hostOf(input: string): string {
  const trimmed = input.trim()
  try {
    const url = new URL(/^[a-z][a-z0-9+.-]*:\/\//i.test(trimmed) ? trimmed : `https://${trimmed}`)
    return url.hostname.replace(/^www\./, '')
  } catch {
    return trimmed
  }
}

/** The sidebar with a placeholder row at the end of `folder`, or outside any folder. */
export function withPendingFeed(sidebar: Sidebar, feed: { slug: string; title: string }, folder: string | null): Sidebar {
  const row: SidebarFeed = { ...feed, url: '', site_url: null, unread: 0, last_error: null, pending: true }
  const inFolder = folder !== null && sidebar.folders.some((f) => f.slug === folder)
  return {
    ...sidebar,
    folders: inFolder
      ? sidebar.folders.map((f) => (f.slug === folder ? { ...f, feeds: [...f.feeds, row] } : f))
      : sidebar.folders,
    uncategorized: inFolder ? sidebar.uncategorized : [...sidebar.uncategorized, row],
  }
}
