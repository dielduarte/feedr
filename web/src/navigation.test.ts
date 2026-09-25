import { describe, expect, it } from 'vitest'
import { moveSelection, neighbour, nearEnd } from './navigation'

const keys = ['e', 'd', 'c', 'b', 'a']

describe('moveSelection', () => {
  it('moves by the given step', () => {
    expect(moveSelection(keys, 'd', 1)).toBe('c')
    expect(moveSelection(keys, 'd', -1)).toBe('e')
  })

  it('stays on the first or last article at the ends', () => {
    expect(moveSelection(keys, 'e', -1)).toBe('e')
    expect(moveSelection(keys, 'a', 1)).toBe('a')
  })

  it('starts from the top when nothing, or something no longer listed, is selected', () => {
    expect(moveSelection(keys, null, 1)).toBe('e')
    expect(moveSelection(keys, 'zz', 1)).toBe('e')
    expect(moveSelection([], null, 1)).toBeNull()
  })
})

describe('neighbour', () => {
  it('finds the next and previous article of the one open', () => {
    expect(neighbour(keys, 'c', 1)).toBe('b')
    expect(neighbour(keys, 'c', -1)).toBe('d')
  })

  it('has no neighbour past the ends or for an article outside the list', () => {
    expect(neighbour(keys, 'a', 1)).toBeNull()
    expect(neighbour(keys, 'e', -1)).toBeNull()
    expect(neighbour(keys, 'zz', 1)).toBeNull()
  })
})

describe('nearEnd', () => {
  it('is true within the last few loaded articles', () => {
    expect(nearEnd(keys, 'b', 3)).toBe(true)
    expect(nearEnd(keys, 'e', 3)).toBe(false)
  })
})
