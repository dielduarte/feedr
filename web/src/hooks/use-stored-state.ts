import { useCallback, useState } from 'react'
import { parseStored, storageKey } from '@/lib/storage'

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
