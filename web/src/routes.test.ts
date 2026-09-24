import { describe, expect, it } from 'vitest'
import { itemPath, parseLocation, scopePath, type Scope } from './routes'

const scopes: Scope[] = [
  { kind: 'all' },
  { kind: 'unread' },
  { kind: 'starred' },
  { kind: 'folder', id: 3 },
  { kind: 'feed', id: 7 },
]

describe('routes', () => {
  it('round-trips every scope through its path', () => {
    for (const scope of scopes) {
      expect(parseLocation(scopePath(scope))).toEqual({ scope, itemId: null })
    }
  })

  it('round-trips an open article within its scope', () => {
    for (const scope of scopes) {
      expect(parseLocation(itemPath(scope, 42))).toEqual({ scope, itemId: 42 })
    }
  })

  it('uses readable paths', () => {
    expect(scopePath({ kind: 'all' })).toBe('/')
    expect(itemPath({ kind: 'all' }, 9)).toBe('/items/9')
    expect(itemPath({ kind: 'folder', id: 3 }, 9)).toBe('/folders/3/items/9')
  })

  it('falls back to all articles for unknown or malformed paths', () => {
    expect(parseLocation('/nope')).toEqual({ scope: { kind: 'all' }, itemId: null })
    expect(parseLocation('/feeds/abc')).toEqual({ scope: { kind: 'all' }, itemId: null })
    expect(parseLocation('/feeds/7/items/x')).toEqual({ scope: { kind: 'feed', id: 7 }, itemId: null })
  })
})
