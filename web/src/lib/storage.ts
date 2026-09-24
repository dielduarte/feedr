import { useCallback, useState } from 'react'

// Bump when a stored value changes shape, so old values are ignored instead of misread.
const VERSION = 'v1'

export function storageKey(name: string): string {
  return `feedrsauros:${VERSION}:${name}`
}

/** Parses a stored value, falling back when it's missing, corrupt, or not the fallback's type. */
export function parseStored<T>(raw: string | null, fallback: T): T {
  if (raw === null) return fallback
  try {
    const value: unknown = JSON.parse(raw)
    const sameShape =
      Array.isArray(fallback) ? Array.isArray(value) : typeof value === typeof fallback && !Array.isArray(value)
    return sameShape ? (value as T) : fallback
  } catch {
    return fallback
  }
}

function load<T>(name: string, fallback: T): T {
  try {
    return parseStored(localStorage.getItem(storageKey(name)), fallback)
  } catch {
    return fallback
  }
}

/** A preference that survives reloads; storage failures keep the value in memory only. */
export function useStoredState<T>(name: string, fallback: T) {
  const [value, setValue] = useState<T>(() => load(name, fallback))
  const update = useCallback(
    (next: T | ((previous: T) => T)) => {
      setValue((previous) => {
        const resolved = next instanceof Function ? next(previous) : next
        try {
          localStorage.setItem(storageKey(name), JSON.stringify(resolved))
        } catch {
          // Private mode or blocked storage.
        }
        return resolved
      })
    },
    [name],
  )
  return [value, update] as const
}
