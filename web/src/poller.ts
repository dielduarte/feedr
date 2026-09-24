import { useQueryClient } from '@tanstack/react-query'
import { useCallback, useEffect, useRef, useState } from 'react'
import { toast } from 'sonner'
import { api, type PollerEvent } from './api'
import { sentence } from './format'
import { keys } from './queries'
import type { Scope } from './routes'

export type PollerStatus = { refreshing: boolean; offline: boolean }

/** How long a requested refresh may wait for the server to start before we stop showing it. */
const REFRESH_START_TIMEOUT = 10_000

/**
 * Follows the server's poller over SSE, refreshes what's on screen as feeds update, and reports
 * whether a refresh is in progress, including one just requested from here.
 */
export function usePoller() {
  const client = useQueryClient()
  const [status, setStatus] = useState<PollerStatus>({ refreshing: false, offline: false })
  const startTimeout = useRef<ReturnType<typeof setTimeout>>(undefined)

  useEffect(() => {
    const source = new EventSource('/api/events')
    source.onmessage = (message) => {
      const event: PollerEvent = JSON.parse(message.data)
      switch (event.type) {
        case 'batch_started':
          clearTimeout(startTimeout.current)
          setStatus((s) => ({ ...s, refreshing: true }))
          break
        case 'feed_refreshed':
        case 'feed_failed':
          client.invalidateQueries({ queryKey: keys.sidebar })
          break
        case 'batch_finished':
          setStatus({ refreshing: false, offline: event.health === 'offline' })
          client.invalidateQueries({ queryKey: keys.sidebar })
          client.invalidateQueries({ queryKey: keys.allItems })
          break
        case 'resync':
          client.invalidateQueries()
          break
      }
    }
    return () => {
      source.close()
      clearTimeout(startTimeout.current)
    }
  }, [client])

  // Shows progress from the click, before the server's batch begins; stops if nothing was due
  // or the server never picks it up.
  const refresh = useCallback(async (scope: Scope) => {
    const stop = () => {
      clearTimeout(startTimeout.current)
      setStatus((s) => ({ ...s, refreshing: false }))
    }
    setStatus((s) => ({ ...s, refreshing: true }))
    clearTimeout(startTimeout.current)
    startTimeout.current = setTimeout(stop, REFRESH_START_TIMEOUT)
    try {
      const { scheduled } = await api.refresh(scope)
      if (scheduled === 0) stop()
    } catch (error) {
      stop()
      toast.error(sentence((error as Error).message))
    }
  }, [])

  return { status, refresh }
}
