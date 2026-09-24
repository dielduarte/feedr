import { Suspense, useCallback, useMemo, useState } from 'react'
import { useLocation } from 'wouter'
import { SidebarInset, SidebarProvider, useSidebar } from '@/components/ui/sidebar'
import { useStoredState } from '@/lib/storage'
import { listsUnreadOnly } from './api'
import type { Chrome } from './chrome'
import { AddFeedDialog, type DialogName, ShortcutsDialog, TransferDialog, usePreloadDialogs } from './components/dialogs'
import { AppSidebar } from './components/sidebar/AppSidebar'
import { scopeLabel } from './lookup'
import { ListPage } from './pages/ListPage'
import { ReaderPage } from './pages/ReaderPage'
import { usePoller } from './poller'
import { useLookup, useSidebarData } from './queries'
import { itemPath, parseLocation, scopePath, type Scope } from './routes'

export function App() {
  const [open, setOpen] = useStoredState('sidebarOpen', true)
  return (
    <SidebarProvider open={open} onOpenChange={setOpen}>
      <Shell />
    </SidebarProvider>
  )
}

function Shell() {
  const [path, setPath] = useLocation()
  // Parsed once per URL so `scope` keeps its identity and memoized children don't re-render.
  const { scope, itemId } = useMemo(() => parseLocation(path), [path])
  const { toggleSidebar, isMobile, setOpenMobile } = useSidebar()
  const [unreadPreference, setUnreadPreference] = useStoredState('unreadOnly', false)
  const [dialog, setDialog] = useState<DialogName | null>(null)
  const [lastOpenedId, setLastOpenedId] = useState<number | null>(null)
  const { status, refresh } = usePoller()
  const { data: sidebar } = useSidebarData()
  const lookup = useLookup(sidebar)
  usePreloadDialogs()

  const navigate = useCallback(
    (target: Scope) => {
      setPath(scopePath(target))
      if (isMobile) setOpenMobile(false)
    },
    [setPath, isMobile, setOpenMobile],
  )
  const openArticle = useCallback(
    (id: number) => {
      setLastOpenedId(id)
      setPath(itemPath(scope, id))
    },
    [setPath, scope],
  )
  const backToList = useCallback(() => setPath(scopePath(scope)), [setPath, scope])
  const closeDialog = useCallback(() => setDialog(null), [])

  const chrome = useMemo<Chrome>(() => {
    const onRefresh = () => refresh(scope.kind === 'feed' || scope.kind === 'folder' ? scope : { kind: 'all' })
    return {
      scope,
      label: scopeLabel(scope, lookup),
      sidebar,
      lookup,
      status,
      onNavigate: navigate,
      onRefresh,
      shortcuts: { r: onRefresh, '[': toggleSidebar, '?': () => setDialog('shortcuts') },
      shortcutsEnabled: dialog === null,
    }
  }, [scope, lookup, sidebar, status, navigate, refresh, toggleSidebar, dialog])

  return (
    <>
      <AppSidebar scope={scope} sidebar={sidebar} onNavigate={navigate} onOpenDialog={setDialog} />

      <SidebarInset className="h-dvh min-w-0 overflow-hidden md:h-[calc(100dvh-1rem)] md:border md:shadow-[0_1px_2px_rgb(0_0_0/0.03),0_18px_40px_-20px_rgb(0_0_0/0.12)]">
        {itemId === null ? (
          <ListPage
            // A new scope starts with a fresh selection.
            key={scopePath(scope)}
            chrome={chrome}
            unreadPreference={unreadPreference}
            onUnreadPreferenceChange={setUnreadPreference}
            initialSelectedId={lastOpenedId}
            onOpen={openArticle}
            onAddFeed={() => setDialog('add')}
            onImport={() => setDialog('transfer')}
          />
        ) : (
          <ReaderPage
            chrome={chrome}
            itemId={itemId}
            unreadOnly={listsUnreadOnly(scope, unreadPreference)}
            onBack={backToList}
            onOpen={openArticle}
          />
        )}
      </SidebarInset>

      <Suspense fallback={null}>
        {dialog === 'add' ? (
          <AddFeedDialog
            sidebar={sidebar}
            defaultFolder={scope.kind === 'folder' ? scope.id : null}
            onClose={closeDialog}
            onAdded={(id) => {
              closeDialog()
              navigate({ kind: 'feed', id })
            }}
          />
        ) : null}
        {dialog === 'transfer' ? <TransferDialog onClose={closeDialog} /> : null}
        {dialog === 'shortcuts' ? <ShortcutsDialog onClose={closeDialog} /> : null}
      </Suspense>
    </>
  )
}
