import { describe, expect, it } from 'vitest'
import type { Sidebar } from './api'
import { buildLookup, scopeLabel } from './lookup'

const feed = (slug: string, title: string) => ({ slug, title, url: `https://${slug}.test/feed`, site_url: null, unread: 0, last_error: null })

const sidebar: Sidebar = {
  total_unread: 0,
  total_starred: 0,
  folders: [{ slug: 'tech', name: 'Tech', unread: 0, feeds: [feed('rust-blog', 'Rust Blog')] }],
  uncategorized: [feed('xkcd', 'xkcd')],
}

describe('buildLookup', () => {
  it('finds feeds in folders and outside them by slug', () => {
    const lookup = buildLookup(sidebar)

    expect(lookup.feed('rust-blog')?.title).toBe('Rust Blog')
    expect(lookup.feed('xkcd')?.title).toBe('xkcd')
    expect(lookup.feed('nope')).toBeUndefined()
    expect(lookup.folder('tech')?.name).toBe('Tech')
  })

  it('knows whether there is anything subscribed', () => {
    expect(buildLookup(sidebar).hasFeeds).toBe(true)
    expect(buildLookup({ total_unread: 0, total_starred: 0, folders: [{ slug: 'empty', name: 'Empty', unread: 0, feeds: [] }], uncategorized: [] }).hasFeeds).toBe(false)
    expect(buildLookup(undefined).hasFeeds).toBe(false)
  })
})

describe('scopeLabel', () => {
  const lookup = buildLookup(sidebar)

  it('names every kind of scope', () => {
    expect(scopeLabel({ kind: 'all' }, lookup)).toBe('All articles')
    expect(scopeLabel({ kind: 'unread' }, lookup)).toBe('Unread')
    expect(scopeLabel({ kind: 'starred' }, lookup)).toBe('Starred')
    expect(scopeLabel({ kind: 'folder', slug: 'tech' }, lookup)).toBe('Tech')
    expect(scopeLabel({ kind: 'feed', slug: 'xkcd' }, lookup)).toBe('xkcd')
  })

  it('falls back to a generic name for unknown folders and feeds', () => {
    expect(scopeLabel({ kind: 'folder', slug: 'nope' }, lookup)).toBe('Folder')
    expect(scopeLabel({ kind: 'feed', slug: 'nope' }, lookup)).toBe('Feed')
  })
})
