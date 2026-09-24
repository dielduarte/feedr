import { describe, expect, it } from 'vitest'
import { moveSelection, neighbour, nearEnd } from './navigation'

const ids = [5, 4, 3, 2, 1]

describe('moveSelection', () => {
  it('moves by the given step', () => {
    expect(moveSelection(ids, 4, 1)).toBe(3)
    expect(moveSelection(ids, 4, -1)).toBe(5)
  })

  it('stays on the first or last article at the ends', () => {
    expect(moveSelection(ids, 5, -1)).toBe(5)
    expect(moveSelection(ids, 1, 1)).toBe(1)
  })

  it('starts from the top when nothing, or something no longer listed, is selected', () => {
    expect(moveSelection(ids, null, 1)).toBe(5)
    expect(moveSelection(ids, 42, 1)).toBe(5)
    expect(moveSelection([], null, 1)).toBeNull()
  })
})

describe('neighbour', () => {
  it('finds the next and previous article of the one open', () => {
    expect(neighbour(ids, 3, 1)).toBe(2)
    expect(neighbour(ids, 3, -1)).toBe(4)
  })

  it('has no neighbour past the ends or for an article outside the list', () => {
    expect(neighbour(ids, 1, 1)).toBeNull()
    expect(neighbour(ids, 5, -1)).toBeNull()
    expect(neighbour(ids, 42, 1)).toBeNull()
  })
})

describe('nearEnd', () => {
  it('is true within the last few loaded articles', () => {
    expect(nearEnd(ids, 2, 3)).toBe(true)
    expect(nearEnd(ids, 5, 3)).toBe(false)
  })
})
