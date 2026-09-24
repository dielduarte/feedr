import { AlertCircle, ChevronRight, CircleDot, FolderInput, Inbox, Keyboard, MoreHorizontal, Plus, Star } from 'lucide-react'
import { type DragEvent, Fragment, type ReactNode, useRef, useState } from 'react'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupAction,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarInput,
  SidebarMenu,
  SidebarMenuAction,
  SidebarMenuBadge,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarTrigger,
} from '@/components/ui/sidebar'
import { useStoredState } from '@/lib/hooks'
import { cn } from '@/lib/utils'
import { api, type Sidebar as SidebarData, type SidebarFeed, type SidebarFolder } from '../api'
import { useSidebarMutation } from '../queries'
import { dropIndex } from '../reorder'
import { scopePath, type Scope } from '../routes'
import { FeedIcon } from './FeedIcon'

type Props = {
  scope: Scope
  sidebar: SidebarData | undefined
  onNavigate: (scope: Scope) => void
  onAddFeed: () => void
  onOpenTransfer: () => void
  onOpenShortcuts: () => void
}

type Dragging = { kind: 'feed'; id: number } | { kind: 'folder'; id: number } | null
type Target = { kind: 'feed'; feed: SidebarFeed } | { kind: 'folder'; folder: SidebarFolder }

const isActive = (current: Scope, candidate: Scope) => scopePath(current) === scopePath(candidate)

const dropLine =
  "before:absolute before:inset-x-2 before:-top-px before:h-0.5 before:rounded-full before:bg-signal before:content-['']"

export function AppSidebar({ scope, sidebar, onNavigate, onAddFeed, onOpenTransfer, onOpenShortcuts }: Props) {
  const [collapsed, setCollapsed] = useStoredState<number[]>('feedr.collapsedFolders', [])
  const [dragging, setDragging] = useState<Dragging>(null)
  const [dropTarget, setDropTarget] = useState<string | null>(null)
  const [renaming, setRenaming] = useState<string | null>(null)
  const [creatingFolder, setCreatingFolder] = useState(false)
  const [removing, setRemoving] = useState<Target | null>(null)

  const moveFeed = useSidebarMutation((a: { id: number; folderId: number | null; index: number }) =>
    api.moveFeed(a.id, a.folderId, a.index),
  )
  const moveFolder = useSidebarMutation((a: { id: number; index: number }) => api.moveFolder(a.id, a.index))
  const renameFeed = useSidebarMutation((a: { id: number; title: string | null }) => api.renameFeed(a.id, a.title))
  const renameFolder = useSidebarMutation((a: { id: number; name: string }) => api.renameFolder(a.id, a.name))
  const createFolder = useSidebarMutation((name: string) => api.createFolder(name))
  const unsubscribe = useSidebarMutation((id: number) => api.unsubscribe(id))
  const deleteFolder = useSidebarMutation((id: number) => api.deleteFolder(id))

  const folders = sidebar?.folders ?? []
  const uncategorized = sidebar?.uncategorized ?? []
  const isEmpty = !!sidebar && folders.length === 0 && uncategorized.length === 0

  const endDrag = () => {
    setDragging(null)
    setDropTarget(null)
  }

  const dropFeed = (folderId: number | null, siblings: SidebarFeed[], before: number | 'end') => {
    if (dragging?.kind === 'feed') {
      moveFeed.mutate({ id: dragging.id, folderId, index: dropIndex(siblings.map((f) => f.id), dragging.id, before) })
    }
    endDrag()
  }

  const dropOnFolder = (folder: SidebarFolder) => {
    if (dragging?.kind === 'feed') return dropFeed(folder.id, folder.feeds, 'end')
    if (dragging?.kind === 'folder' && dragging.id !== folder.id) {
      moveFolder.mutate({ id: dragging.id, index: dropIndex(folders.map((f) => f.id), dragging.id, folder.id) })
    }
    endDrag()
  }

  const toggleFolder = (id: number) =>
    setCollapsed((ids) => (ids.includes(id) ? ids.filter((x) => x !== id) : [...ids, id]))

  const dragSource = (item: Exclude<Dragging, null>, name: string) => ({
    draggable: renaming === null,
    onDragStart: (event: DragEvent) => {
      event.dataTransfer.effectAllowed = 'move'
      event.dataTransfer.setData('text/plain', name)
      setDragging(item)
    },
    onDragEnd: endDrag,
  })

  const dropZone = (key: string, accepts: boolean, onDrop: () => void) => ({
    onDragOver: (event: DragEvent) => {
      if (!accepts) return
      event.preventDefault()
      event.stopPropagation()
      setDropTarget(key)
    },
    onDragLeave: () => setDropTarget((current) => (current === key ? null : current)),
    onDrop: (event: DragEvent) => {
      event.preventDefault()
      event.stopPropagation()
      onDrop()
    },
  })

  const feedItem = (feed: SidebarFeed, folderId: number | null, siblings: SidebarFeed[]) => {
    const key = `feed:${feed.id}`
    const feedScope: Scope = { kind: 'feed', id: feed.id }
    return (
      <SidebarMenuItem
        key={feed.id}
        className={cn(folderId !== null && 'pl-4', dropTarget === key && dropLine)}
        {...dragSource({ kind: 'feed', id: feed.id }, feed.title)}
        {...dropZone(key, dragging?.kind === 'feed' && dragging.id !== feed.id, () => dropFeed(folderId, siblings, feed.id))}
      >
        {renaming === key ? (
          <RenameInput
            initial={feed.title}
            label="Feed name"
            onDone={(title) => {
              if (title !== undefined) renameFeed.mutate({ id: feed.id, title: title || null })
              setRenaming(null)
            }}
          />
        ) : (
          <>
            <SidebarMenuButton isActive={isActive(scope, feedScope)} onClick={() => onNavigate(feedScope)} className="text-muted-foreground data-[active=true]:text-foreground hover:text-foreground">
              <FeedIcon siteUrl={feed.site_url} />
              <span className="truncate">{feed.title}</span>
              {feed.last_error && (
                <AlertCircle className="text-warning" aria-label={`Not updating: ${feed.last_error}`} />
              )}
            </SidebarMenuButton>
            <Count value={feed.unread} />
            <RowMenu
              label={feed.title}
              onRename={() => setRenaming(key)}
              onRefresh={() => api.refresh(feedScope)}
              destructiveLabel="Unsubscribe…"
              onDestroy={() => setRemoving({ kind: 'feed', feed })}
            />
          </>
        )}
      </SidebarMenuItem>
    )
  }

  return (
    <>
      <Sidebar variant="inset" collapsible="offcanvas">
        <SidebarHeader className="h-12 flex-row items-center">
          <SidebarTrigger className="text-muted-foreground" />
        </SidebarHeader>

        <SidebarContent>
          <SidebarGroup className="mt-4">
            <SidebarMenu>
              <NavItem scope={scope} target={{ kind: 'all' }} icon={<Inbox />} label="All articles" onNavigate={onNavigate} />
              <NavItem scope={scope} target={{ kind: 'unread' }} icon={<CircleDot />} label="Unread" count={sidebar?.total_unread} onNavigate={onNavigate} />
              <NavItem scope={scope} target={{ kind: 'starred' }} icon={<Star />} label="Starred" onNavigate={onNavigate} />
            </SidebarMenu>
          </SidebarGroup>

          <SidebarGroup className="mt-4">
            <SidebarGroupLabel className="text-[13px] font-medium text-foreground">Subscriptions</SidebarGroupLabel>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <SidebarGroupAction aria-label="Add a feed or folder">
                  <Plus />
                </SidebarGroupAction>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="dark">
                <DropdownMenuItem onSelect={onAddFeed}>Add feed</DropdownMenuItem>
                <DropdownMenuItem onSelect={() => setCreatingFolder(true)}>New folder</DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>

            <SidebarGroupContent>
              <SidebarMenu>
                {creatingFolder && (
                  <SidebarMenuItem>
                    <RenameInput
                      initial=""
                      label="Folder name"
                      onDone={(name) => {
                        if (name) createFolder.mutate(name)
                        setCreatingFolder(false)
                      }}
                    />
                  </SidebarMenuItem>
                )}

                {folders.map((folder) => {
                  const key = `folder:${folder.id}`
                  const folderScope: Scope = { kind: 'folder', id: folder.id }
                  const open = !collapsed.includes(folder.id)
                  const hovered = dropTarget === key
                  return (
                    // Feed rows are siblings of their folder row, not children, so hover and
                    // menu state on one row never leaks into another through `group` variants.
                    <Fragment key={folder.id}>
                      <SidebarMenuItem
                        className={cn(hovered && dragging?.kind === 'folder' && dropLine)}
                        {...dragSource({ kind: 'folder', id: folder.id }, folder.name)}
                        {...dropZone(
                          key,
                          dragging !== null && !(dragging.kind === 'folder' && dragging.id === folder.id),
                          () => dropOnFolder(folder),
                        )}
                      >
                        {renaming === key ? (
                          <RenameInput
                            initial={folder.name}
                            label="Folder name"
                            onDone={(name) => {
                              if (name) renameFolder.mutate({ id: folder.id, name })
                              setRenaming(null)
                            }}
                          />
                        ) : (
                          <>
                            <SidebarMenuButton
                              isActive={isActive(scope, folderScope)}
                              onClick={() => onNavigate(folderScope)}
                              className={cn('pl-8', hovered && dragging?.kind === 'feed' && 'bg-signal/12')}
                            >
                              <span className="truncate">{folder.name}</span>
                            </SidebarMenuButton>
                            <button
                              type="button"
                              aria-expanded={open}
                              aria-label={open ? `Collapse ${folder.name}` : `Expand ${folder.name}`}
                              onClick={() => toggleFolder(folder.id)}
                              className="absolute top-1.5 left-1.5 grid size-5 place-items-center rounded-sm text-faint hover:text-foreground"
                            >
                              <ChevronRight className={cn('size-3.5 transition-transform motion-reduce:transition-none', open && 'rotate-90')} />
                            </button>
                            <Count value={folder.unread} />
                            <RowMenu
                              label={folder.name}
                              onRename={() => setRenaming(key)}
                              onRefresh={() => api.refresh(folderScope)}
                              destructiveLabel="Delete folder…"
                              onDestroy={() => setRemoving({ kind: 'folder', folder })}
                            />
                          </>
                        )}
                      </SidebarMenuItem>
                      {open && folder.feeds.map((feed) => feedItem(feed, folder.id, folder.feeds))}
                    </Fragment>
                  )
                })}

                {uncategorized.map((feed) => feedItem(feed, null, uncategorized))}

                {dragging?.kind === 'feed' && (
                  <li
                    className={cn(
                      'mt-2 flex items-center gap-2 rounded-md border border-dashed p-2.5 text-xs text-muted-foreground',
                      dropTarget === 'uncategorized' ? 'border-signal bg-signal/8' : 'border-faint',
                    )}
                    {...dropZone('uncategorized', true, () => dropFeed(null, uncategorized, 'end'))}
                  >
                    <FolderInput className="size-3.5" /> Drop here to take it out of its folder
                  </li>
                )}

                {isEmpty && !creatingFolder && (
                  <SidebarMenuItem>
                    <SidebarMenuButton onClick={onAddFeed} className="text-muted-foreground">
                      <Plus />
                      Add your first feed
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                )}
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>
        </SidebarContent>

        <SidebarFooter>
          <SidebarMenu>
            <SidebarMenuItem>
              <SidebarMenuButton onClick={onOpenTransfer}>
                <FolderInput />
                Import & export
              </SidebarMenuButton>
            </SidebarMenuItem>
            <SidebarMenuItem>
              <SidebarMenuButton onClick={onOpenShortcuts}>
                <Keyboard />
                Keyboard shortcuts
              </SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>
        </SidebarFooter>
      </Sidebar>

      <AlertDialog open={removing !== null} onOpenChange={(open) => !open && setRemoving(null)}>
        <AlertDialogContent className="dark bg-popover text-popover-foreground sm:max-w-sm">
          {removing?.kind === 'feed' && (
            <AlertDialogHeader>
              <AlertDialogTitle>Unsubscribe from {removing.feed.title}?</AlertDialogTitle>
              <AlertDialogDescription>Its articles are removed too, including starred ones.</AlertDialogDescription>
            </AlertDialogHeader>
          )}
          {removing?.kind === 'folder' && (
            <AlertDialogHeader>
              <AlertDialogTitle>Delete {removing.folder.name}?</AlertDialogTitle>
              <AlertDialogDescription>Its feeds stay subscribed, outside any folder.</AlertDialogDescription>
            </AlertDialogHeader>
          )}
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              className="bg-destructive text-background hover:bg-destructive/90"
              onClick={() => {
                if (removing?.kind === 'feed') {
                  unsubscribe.mutate(removing.feed.id)
                  if (isActive(scope, { kind: 'feed', id: removing.feed.id })) onNavigate({ kind: 'all' })
                }
                if (removing?.kind === 'folder') {
                  deleteFolder.mutate(removing.folder.id)
                  if (isActive(scope, { kind: 'folder', id: removing.folder.id })) onNavigate({ kind: 'all' })
                }
              }}
            >
              {removing?.kind === 'feed' ? 'Unsubscribe' : 'Delete folder'}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  )
}

type NavItemProps = {
  scope: Scope
  target: Scope
  icon: ReactNode
  label: string
  count?: number
  onNavigate: (scope: Scope) => void
}

function NavItem({ scope, target, icon, label, count, onNavigate }: NavItemProps) {
  return (
    <SidebarMenuItem>
      <SidebarMenuButton isActive={isActive(scope, target)} onClick={() => onNavigate(target)}>
        {icon}
        {label}
      </SidebarMenuButton>
      {count !== undefined && count > 0 && <SidebarMenuBadge className="font-normal text-faint tabular-nums">{count}</SidebarMenuBadge>}
    </SidebarMenuItem>
  )
}

/** Unread count that makes room for the row's menu button on hover. */
function Count({ value }: { value: number }) {
  if (value === 0) return null
  return (
    <SidebarMenuBadge className="font-normal text-faint tabular-nums group-focus-within/menu-item:opacity-0 group-hover/menu-item:opacity-0 group-has-data-[state=open]/menu-item:opacity-0">
      {value}
    </SidebarMenuBadge>
  )
}

type RowMenuProps = {
  label: string
  onRename: () => void
  onRefresh: () => void
  destructiveLabel: string
  onDestroy: () => void
}

function RowMenu({ label, onRename, onRefresh, destructiveLabel, onDestroy }: RowMenuProps) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <SidebarMenuAction showOnHover aria-label={`Options for ${label}`}>
          <MoreHorizontal />
        </SidebarMenuAction>
      </DropdownMenuTrigger>
      <DropdownMenuContent side="right" align="start" className="dark">
        <DropdownMenuItem onSelect={onRename}>Rename</DropdownMenuItem>
        <DropdownMenuItem onSelect={onRefresh}>Refresh now</DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem variant="destructive" onSelect={onDestroy}>
          {destructiveLabel}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

type RenameInputProps = {
  initial: string
  label: string
  /** Receives the trimmed name, or `undefined` when cancelled. */
  onDone: (value: string | undefined) => void
}

function RenameInput({ initial, label, onDone }: RenameInputProps) {
  const [value, setValue] = useState(initial)
  // Enter finishes and unmounts the input, which also fires blur; only the first one counts.
  const finished = useRef(false)
  const finish = (result: string | undefined) => {
    if (finished.current) return
    finished.current = true
    onDone(result)
  }
  return (
    <SidebarInput
      aria-label={label}
      placeholder={label}
      autoFocus
      value={value}
      onChange={(event) => setValue(event.target.value)}
      onFocus={(event) => event.target.select()}
      onKeyDown={(event) => {
        if (event.key === 'Enter') finish(value.trim())
        if (event.key === 'Escape') finish(undefined)
      }}
      onBlur={() => finish(value.trim())}
      className="h-8 border-signal bg-background"
    />
  )
}
