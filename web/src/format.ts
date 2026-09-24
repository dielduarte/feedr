const MINUTE = 60_000
const HOUR = 60 * MINUTE
const DAY = 24 * HOUR
const WORDS_PER_MINUTE = 230

const longDate = new Intl.DateTimeFormat('en-US', { dateStyle: 'long' })

/** Compact age ("5m", "3h", "2w", "5mo"), at most four characters so dates line up in a column. */
export function relativeTime(date: Date, now: Date): string {
  const elapsed = now.getTime() - date.getTime()
  if (elapsed < MINUTE) return 'now'
  if (elapsed < HOUR) return `${Math.floor(elapsed / MINUTE)}m`
  if (elapsed < DAY) return `${Math.floor(elapsed / HOUR)}h`
  const days = Math.floor(elapsed / DAY)
  if (days < 7) return `${days}d`
  if (days < 30) return `${Math.floor(days / 7)}w`
  if (days < 365) return `${Math.floor(days / 30)}mo`
  return `${Math.floor(days / 365)}y`
}

export function fullDate(date: Date): string {
  return longDate.format(date)
}

export function wordCount(html: string): number {
  const text = new DOMParser().parseFromString(html, 'text/html').body.textContent ?? ''
  return text.split(/\s+/).filter(Boolean).length
}

export function readingMinutes(words: number): number {
  return Math.max(1, Math.round(words / WORDS_PER_MINUTE))
}
