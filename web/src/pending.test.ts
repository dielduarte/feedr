import { describe, expect, it } from 'vitest'
import type { Sidebar } from './api'
import { hostOf, isPendingSlug, pendingSlug, withPendingFeed } from './pending'

const feed = (slug: string) => ({ slug, title: slug, url: `https://${slug}.test/`, site_url: null, unread: 0, last_error: null })

const sidebar: Sidebar = {
  total_unread: 0,
  folders: [{ slug: 'tech', name: 'Tech', unread: 0, feeds: [feed('rust-blog')] }],
  uncategorized: [feed('xkcd')],
}

describe('pending feeds', () => {
  it('get slugs that can never clash with a real one', () => {
    const slug = pendingSlug()

    expect(isPendingSlug(slug)).toBe(true)
    expect(isPendingSlug('rust-blog')).toBe(false)
    expect(pendingSlug()).not.toBe(slug)
  })

  it('are named after the site being added', () => {
    expect(hostOf('https://www.example.com/blog/feed.xml')).toBe('example.com')
    expect(hostOf('  jvns.ca ')).toBe('jvns.ca')
    expect(hostOf('not a url')).toBe('not a url')
  })

  it('appear at the end of the chosen folder', () => {
    const next = withPendingFeed(sidebar, { slug: '~adding-1', title: 'example.com' }, 'tech')

    expect(next.folders[0].feeds.map((f) => f.slug)).toEqual(['rust-blog', '~adding-1'])
    expect(next.folders[0].feeds[1].pending).toBe(true)
    expect(next.uncategorized.map((f) => f.slug)).toEqual(['xkcd'])
    expect(sidebar.folders[0].feeds).toHaveLength(1)
  })

  it('appear outside any folder when none, or an unknown one, is chosen', () => {
    expect(withPendingFeed(sidebar, { slug: '~adding-1', title: 'a' }, null).uncategorized.map((f) => f.slug)).toEqual(['xkcd', '~adding-1'])
    expect(withPendingFeed(sidebar, { slug: '~adding-1', title: 'a' }, 'nope').uncategorized).toHaveLength(2)
  })
})
