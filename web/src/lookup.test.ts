import { describe, expect, it } from 'vitest'
import type { Sidebar } from './api'
import { buildLookup, scopeLabel } from './lookup'

const feed = (id: number, title: string) => ({ id, title, url: `https://${id}.test/feed`, site_url: null, unread: 0, last_error: null })

const sidebar: Sidebar = {
  total_unread: 0,
  folders: [{ id: 1, name: 'Tech', unread: 0, feeds: [feed(10, 'Rust Blog')] }],
  uncategorized: [feed(20, 'xkcd')],
}

describe('buildLookup', () => {
  it('finds feeds in folders and outside them', () => {
    const lookup = buildLookup(sidebar)

    expect(lookup.feed(10)?.title).toBe('Rust Blog')
    expect(lookup.feed(20)?.title).toBe('xkcd')
    expect(lookup.feed(99)).toBeUndefined()
    expect(lookup.folder(1)?.name).toBe('Tech')
  })

  it('knows whether there is anything subscribed', () => {
    expect(buildLookup(sidebar).hasFeeds).toBe(true)
    expect(buildLookup({ total_unread: 0, folders: [{ id: 1, name: 'Empty', unread: 0, feeds: [] }], uncategorized: [] }).hasFeeds).toBe(false)
    expect(buildLookup(undefined).hasFeeds).toBe(false)
  })
})

describe('scopeLabel', () => {
  const lookup = buildLookup(sidebar)

  it('names every kind of scope', () => {
    expect(scopeLabel({ kind: 'all' }, lookup)).toBe('All articles')
    expect(scopeLabel({ kind: 'unread' }, lookup)).toBe('Unread')
    expect(scopeLabel({ kind: 'starred' }, lookup)).toBe('Starred')
    expect(scopeLabel({ kind: 'folder', id: 1 }, lookup)).toBe('Tech')
    expect(scopeLabel({ kind: 'feed', id: 20 }, lookup)).toBe('xkcd')
  })

  it('falls back to a generic name for unknown folders and feeds', () => {
    expect(scopeLabel({ kind: 'folder', id: 99 }, lookup)).toBe('Folder')
    expect(scopeLabel({ kind: 'feed', id: 99 }, lookup)).toBe('Feed')
  })
})
