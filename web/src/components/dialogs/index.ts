import { lazy, useEffect } from 'react'

export type DialogName = 'add' | 'transfer' | 'shortcuts'

// Dialogs are rarely open, so they live in their own chunks instead of the startup bundle.
const load = {
  add: () => import('./AddFeedDialog'),
  transfer: () => import('./TransferDialog'),
  shortcuts: () => import('./ShortcutsDialog'),
}

export const AddFeedDialog = lazy(() => load.add().then((m) => ({ default: m.AddFeedDialog })))
export const TransferDialog = lazy(() => load.transfer().then((m) => ({ default: m.TransferDialog })))
export const ShortcutsDialog = lazy(() => load.shortcuts().then((m) => ({ default: m.ShortcutsDialog })))

/** Fetches the dialog chunks once the browser is idle, so opening one is instant. */
export function usePreloadDialogs() {
  useEffect(() => {
    const preload = () => Object.values(load).forEach((chunk) => void chunk())
    if ('requestIdleCallback' in window) {
      const handle = requestIdleCallback(preload)
      return () => cancelIdleCallback(handle)
    }
    const timer = setTimeout(preload, 1_000)
    return () => clearTimeout(timer)
  }, [])
}
