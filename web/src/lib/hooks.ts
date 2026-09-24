import { useEffect, useRef, useState } from 'react'

function isTyping(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName))
  )
}

export type Shortcuts = Record<string, () => void>

/**
 * Single-key shortcuts, ignored while typing or when a modifier other than Shift is held. The
 * listener is attached once; the latest bindings are read through a ref.
 */
export function useShortcuts(bindings: Shortcuts, enabled = true) {
  const latest = useRef(bindings)
  useEffect(() => {
    latest.current = bindings
  })

  useEffect(() => {
    if (!enabled) return
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.metaKey || event.ctrlKey || event.altKey || isTyping(event.target)) return
      const action = latest.current[event.key]
      if (action) {
        event.preventDefault()
        action()
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [enabled])
}

/** Re-renders every minute so relative times like "5m" stay current. */
export function useNow(): Date {
  const [now, setNow] = useState(() => new Date())
  useEffect(() => {
    const timer = setInterval(() => setNow(new Date()), 60_000)
    return () => clearInterval(timer)
  }, [])
  return now
}

export function useDocumentTitle(title: string, unread: number) {
  useEffect(() => {
    document.title = `${unread > 0 ? `(${unread}) ` : ''}${title} — feedrsauros`
  }, [title, unread])
}
