import { useCallback, useEffect, useRef, useState } from 'react'

function isTyping(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName))
  )
}

/** Single-key shortcuts, ignored while typing or when a modifier other than Shift is held. */
export function useShortcuts(bindings: Record<string, () => void>, enabled = true) {
  const latest = useRef(bindings)
  latest.current = bindings

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

function read<T>(key: string, fallback: T): T {
  try {
    const stored = localStorage.getItem(key)
    return stored === null ? fallback : (JSON.parse(stored) as T)
  } catch {
    return fallback
  }
}

/** A preference that survives reloads; storage failures just fall back to the default. */
export function useStoredState<T>(key: string, fallback: T) {
  const [value, setValue] = useState<T>(() => read(key, fallback))
  const update = useCallback(
    (next: T | ((previous: T) => T)) => {
      setValue((previous) => {
        const resolved = next instanceof Function ? next(previous) : next
        try {
          localStorage.setItem(key, JSON.stringify(resolved))
        } catch {
          // Private mode or blocked storage: keep the in-memory value.
        }
        return resolved
      })
    },
    [key],
  )
  return [value, update] as const
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
