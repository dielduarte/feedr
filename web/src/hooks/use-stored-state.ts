import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useCallback } from 'react'
import { parseStored, storageKey } from '@/lib/storage'

function load<T>(name: string, fallback: T): T {
  try {
    return parseStored(localStorage.getItem(storageKey(name)), fallback)
  } catch {
    return fallback
  }
}

/**
 * A preference that survives reloads. It lives in the query cache so every component reading the
 * same name updates together; storage failures keep the value in memory only.
 */
export function useStoredState<T>(name: string, fallback: T) {
  const client = useQueryClient()
  const { data = fallback } = useQuery({
    queryKey: ['stored', name],
    queryFn: () => load(name, fallback),
    initialData: () => load(name, fallback),
    staleTime: Infinity,
    gcTime: Infinity,
  })
  const update = useCallback(
    (next: T | ((previous: T) => T)) => {
      // The query exists while any component using it is mounted, so the cached value is current.
      const resolved = next instanceof Function ? next(client.getQueryData<T>(['stored', name]) as T) : next
      client.setQueryData<T>(['stored', name], resolved)
      try {
        localStorage.setItem(storageKey(name), JSON.stringify(resolved))
      } catch {
        // Private mode or blocked storage.
      }
    },
    [client, name],
  )
  return [data, update] as const
}
