import { describe, expect, it } from 'vitest'
import { fullDate, readingMinutes, relativeTime, wordCount } from './format'

const now = new Date('2026-09-23T12:00:00Z')
const ago = (ms: number) => new Date(now.getTime() - ms)
const MIN = 60_000
const HOUR = 60 * MIN
const DAY = 24 * HOUR

describe('relativeTime', () => {
  it('uses one compact format at every age', () => {
    expect(relativeTime(ago(20_000), now)).toBe('now')
    expect(relativeTime(ago(5 * MIN), now)).toBe('5m')
    expect(relativeTime(ago(3 * HOUR), now)).toBe('3h')
    expect(relativeTime(ago(2 * DAY), now)).toBe('2d')
    expect(relativeTime(ago(20 * DAY), now)).toBe('2w')
    expect(relativeTime(ago(150 * DAY), now)).toBe('5mo')
    expect(relativeTime(ago(800 * DAY), now)).toBe('2y')
  })

  it('never needs more than four characters, so dates line up in a column', () => {
    const ages = [0, MIN, 59 * MIN, 23 * HOUR, 6 * DAY, 29 * DAY, 364 * DAY, 99 * 365 * DAY]
    for (const age of ages) expect(relativeTime(ago(age), now).length).toBeLessThanOrEqual(4)
  })

  it('treats future dates as just published', () => {
    expect(relativeTime(new Date(now.getTime() + HOUR), now)).toBe('now')
  })
})

describe('fullDate', () => {
  it('spells out the date for tooltips', () => {
    expect(fullDate(new Date('2025-10-09T12:00:00Z'))).toBe('October 9, 2025')
  })
})

describe('reading stats', () => {
  it('counts words in the rendered text, not the markup', () => {
    expect(wordCount('<p>Hello <a href="/x">big</a>\n world</p><img src="a.png">')).toBe(3)
    expect(wordCount('')).toBe(0)
  })

  it('rounds reading time to whole minutes, at least one', () => {
    expect(readingMinutes(10)).toBe(1)
    expect(readingMinutes(1150)).toBe(5)
  })
})
