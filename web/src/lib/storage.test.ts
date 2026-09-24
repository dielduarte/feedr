import { describe, expect, it } from 'vitest'
import { parseStored, storageKey } from './storage'

describe('stored preferences', () => {
  it('are namespaced and versioned', () => {
    expect(storageKey('sidebarOpen')).toBe('feedrsauros:v1:sidebarOpen')
  })

  it('read back values of the expected type', () => {
    expect(parseStored('false', true)).toBe(false)
    expect(parseStored('[1,2]', [] as number[])).toEqual([1, 2])
  })

  it('fall back when missing, corrupt, or of the wrong type', () => {
    expect(parseStored(null, true)).toBe(true)
    expect(parseStored('{nope', true)).toBe(true)
    expect(parseStored('"yes"', true)).toBe(true)
    expect(parseStored('{"a":1}', [] as number[])).toEqual([])
  })
})
