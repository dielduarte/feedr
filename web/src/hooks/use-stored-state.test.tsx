import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act, renderHook, waitFor } from '@testing-library/react'
import type { ReactNode } from 'react'
import { beforeEach, describe, expect, it } from 'vitest'
import { storageKey } from '@/lib/storage'
import { useStoredState } from './use-stored-state'

let client: QueryClient
const wrapper = ({ children }: { children: ReactNode }) => <QueryClientProvider client={client}>{children}</QueryClientProvider>

beforeEach(() => {
  localStorage.clear()
  client = new QueryClient()
})

describe('stored state', () => {
  it('starts from what was saved', () => {
    localStorage.setItem(storageKey('unreadOnly'), 'true')

    const { result } = renderHook(() => useStoredState('unreadOnly', false), { wrapper })

    expect(result.current[0]).toBe(true)
  })

  it('updates every component using the same name', async () => {
    const { result } = renderHook(
      () => ({ a: useStoredState('collapsedFolders', [] as string[]), b: useStoredState('collapsedFolders', [] as string[]) }),
      { wrapper },
    )

    act(() => result.current.a[1]((previous) => [...previous, 'tech']))

    await waitFor(() => expect(result.current.b[0]).toEqual(['tech']))
  })

  it('survives a reload', () => {
    const first = renderHook(() => useStoredState('sidebarOpen', true), { wrapper })
    act(() => first.result.current[1](false))
    first.unmount()

    client = new QueryClient()
    const { result } = renderHook(() => useStoredState('sidebarOpen', true), { wrapper })

    expect(result.current[0]).toBe(false)
  })
})
