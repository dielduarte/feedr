import { describe, expect, it } from 'vitest'
import { dropIndex } from './reorder'

describe('dropIndex', () => {
  it('inserts before the target in another folder', () => {
    expect(dropIndex([1, 2, 3], 9, 2)).toBe(1)
  })

  it('accounts for the dragged feed leaving its own folder first', () => {
    expect(dropIndex([1, 2, 3], 1, 3)).toBe(1)
    expect(dropIndex([1, 2, 3], 3, 1)).toBe(0)
  })

  it('appends when dropped on the folder itself', () => {
    expect(dropIndex([1, 2, 3], 9, 'end')).toBe(3)
    expect(dropIndex([1, 2, 3], 2, 'end')).toBe(2)
  })
})
