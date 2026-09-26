import { type InfiniteData, QueryClient } from '@tanstack/react-query'
import { beforeEach, describe, expect, it } from 'vitest'
import type { ItemSummary, Page } from './api'
import { keys, removeFromList } from './queries'

const item = (feed: string, slug: string): ItemSummary => ({
  slug,
  feed_slug: feed,
  feed_title: feed,
  url: null,
  title: slug,
  author: null,
  summary: null,
  published_at: '2026-09-25T00:00:00Z',
  fetched_at: '2026-09-25T00:00:00Z',
  read_at: null,
  starred_at: '2026-09-25T00:00:00Z',
})

const pages = (...items: ItemSummary[][]): InfiniteData<Page> => ({
  pages: items.map((page) => ({ items: page, next_cursor: null })),
  pageParams: items.map(() => null),
})

const slugs = (data: InfiniteData<Page> | undefined) => data?.pages.flatMap((page) => page.items.map((i) => i.slug))

let client: QueryClient
beforeEach(() => (client = new QueryClient()))

describe('removing an article from a list', () => {
  it('takes it out of every cached page of that list', () => {
    const starred = keys.items({ kind: 'starred' }, false)
    client.setQueryData(starred, pages([item('rust', 'a'), item('rust', 'b')], [item('go', 'b')]))

    removeFromList(client, { kind: 'starred' }, { feed: 'rust', slug: 'b' })

    expect(slugs(client.getQueryData(starred))).toEqual(['a', 'b'])
    expect(client.getQueryData<InfiniteData<Page>>(starred)?.pages[1].items[0].feed_slug).toBe('go')
  })

  it('leaves other lists alone', () => {
    const all = keys.items({ kind: 'all' }, false)
    client.setQueryData(all, pages([item('rust', 'a')]))

    removeFromList(client, { kind: 'starred' }, { feed: 'rust', slug: 'a' })

    expect(slugs(client.getQueryData(all))).toEqual(['a'])
  })
})
