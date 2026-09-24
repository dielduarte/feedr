import { describe, expect, it } from 'vitest'
import { readingMinutes, relativeTime, wordCount } from './format'

const now = new Date('2026-09-23T12:00:00Z')
const ago = (ms: number) => new Date(now.getTime() - ms)
const MIN = 60_000
const HOUR = 60 * MIN
const DAY = 24 * HOUR

describe('relativeTime', () => {
  it('is compact for recent dates', () => {
    expect(relativeTime(ago(20_000), now)).toBe('Just now')
    expect(relativeTime(ago(5 * MIN), now)).toBe('5m')
    expect(relativeTime(ago(3 * HOUR), now)).toBe('3h')
    expect(relativeTime(ago(2 * DAY), now)).toBe('2d')
  })

  it('shows the date after a week, with the year only when it differs', () => {
    expect(relativeTime(new Date('2026-09-03T12:00:00Z'), now)).toBe('Sep 3')
    expect(relativeTime(new Date('2025-12-24T12:00:00Z'), now)).toBe('Dec 24, 2025')
  })

  it('treats future dates as just published', () => {
    expect(relativeTime(new Date(now.getTime() + HOUR), now)).toBe('Just now')
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
