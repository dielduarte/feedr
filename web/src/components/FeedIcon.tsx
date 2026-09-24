import { Rss } from 'lucide-react'
import { useState } from 'react'

/** The site's favicon, loaded by the browser like any article image, or a feed glyph. */
export function FeedIcon({ siteUrl }: { siteUrl: string | null }) {
  const [failed, setFailed] = useState(false)
  const origin = siteUrl ? safeOrigin(siteUrl) : null

  if (!origin || failed) return <Rss className="size-4 shrink-0 p-px text-ink-3" aria-hidden />
  return (
    <img
      className="size-4 shrink-0 rounded object-cover"
      src={`${origin}/favicon.ico`}
      alt=""
      loading="lazy"
      onError={() => setFailed(true)}
    />
  )
}

function safeOrigin(url: string): string | null {
  try {
    return new URL(url).origin
  } catch {
    return null
  }
}
