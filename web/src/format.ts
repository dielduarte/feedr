const MINUTE = 60_000
const HOUR = 60 * MINUTE
const DAY = 24 * HOUR
const WORDS_PER_MINUTE = 230

const monthDay = new Intl.DateTimeFormat('en-US', { month: 'short', day: 'numeric' })
const monthDayYear = new Intl.DateTimeFormat('en-US', {
  month: 'short',
  day: 'numeric',
  year: 'numeric',
})

export function relativeTime(date: Date, now: Date): string {
  const elapsed = now.getTime() - date.getTime()
  if (elapsed < MINUTE) return 'Just now'
  if (elapsed < HOUR) return `${Math.floor(elapsed / MINUTE)}m`
  if (elapsed < DAY) return `${Math.floor(elapsed / HOUR)}h`
  if (elapsed < 7 * DAY) return `${Math.floor(elapsed / DAY)}d`
  const format = date.getFullYear() === now.getFullYear() ? monthDay : monthDayYear
  return format.format(date)
}

export function wordCount(html: string): number {
  const text = new DOMParser().parseFromString(html, 'text/html').body.textContent ?? ''
  return text.split(/\s+/).filter(Boolean).length
}

export function readingMinutes(words: number): number {
  return Math.max(1, Math.round(words / WORDS_PER_MINUTE))
}
