import { AlertCircle, ChevronRight, CircleDot, FolderInput, Inbox, Keyboard, Plus, Star } from 'lucide-react'
import { memo, type ReactNode, useState } from 'react'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupAction,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarTrigger,
} from '@/components/ui/sidebar'
import { useStoredState } from '@/hooks/use-stored-state'
import { cn } from '@/lib/utils'
import { api, type Sidebar as SidebarData, type SidebarFeed, type SidebarFolder } from '../../api'
import { useSubscriptionActions } from '../../queries'
import { scopePath, type Scope } from '../../routes'
import type { DialogName } from '../dialogs'
import { FeedIcon } from '../FeedIcon'
import { RemoveDialog, type Removal } from './RemoveDialog'
import { RenameInput } from './RenameInput'
import { RowMenu, UnreadCount } from './RowMenu'
import { useTreeDrag } from './useTreeDrag'

type Props = {
  scope: Scope
  sidebar: SidebarData | undefined
  onNavigate: (scope: Scope) => void
  onOpenDialog: (dialog: DialogName) => void
}

const dropLine =
  "before:absolute before:inset-x-2 before:-top-px before:h-0.5 before:rounded-full before:bg-signal before:content-['']"

const isActive = (current: Scope, candidate: Scope) => scopePath(current) === scopePath(candidate)

/** Memoized: it only depends on the sidebar data and where you are, not on the article list. */
export const AppSidebar = memo(function AppSidebar({ scope, sidebar, onNavigate, onOpenDialog }: Props) {
  const actions = useSubscriptionActions()
  const [collapsed, setCollapsed] = useStoredState<number[]>('collapsedFolders', [])
  const [renaming, setRenaming] = useState<string | null>(null)
  const [creatingFolder, setCreatingFolder] = useState(false)
  const [removal, setRemoval] = useState<Removal | null>(null)

  const folders = sidebar?.folders ?? []
  const uncategorized = sidebar?.uncategorized ?? []
  const drag = useTreeDrag({ folders, uncategorized, onMoveFeed: actions.moveFeed, onMoveFolder: actions.moveFolder })

  const toggleFolder = (id: number) =>
    setCollapsed((ids) => (ids.includes(id) ? ids.filter((x) => x !== id) : [...ids, id]))

  const remove = (target: Removal) => {
    if (target.kind === 'feed') {
      actions.unsubscribe(target.feed.id)
      if (isActive(scope, { kind: 'feed', id: target.feed.id })) onNavigate({ kind: 'all' })
    } else {
      actions.deleteFolder(target.folder.id)
      if (isActive(scope, { kind: 'folder', id: target.folder.id })) onNavigate({ kind: 'all' })
    }
    setRemoval(null)
  }

  const feedRow = (feed: SidebarFeed, folderId: number | null, siblings: SidebarFeed[]) => (
    <FeedRow
      key={feed.id}
      feed={feed}
      nested={folderId !== null}
      active={isActive(scope, { kind: 'feed', id: feed.id })}
      renaming={renaming === `feed:${feed.id}`}
      dropBefore={drag.dropsBefore(`feed:${feed.id}`)}
      dragProps={drag.feed(feed, folderId, siblings)}
      onOpen={() => onNavigate({ kind: 'feed', id: feed.id })}
      onRename={() => setRenaming(`feed:${feed.id}`)}
      onRenamed={(title) => {
        if (title !== undefined) actions.renameFeed({ id: feed.id, title: title || null })
        setRenaming(null)
      }}
      onRemove={() => setRemoval({ kind: 'feed', feed })}
    />
  )

  return (
    <>
      <Sidebar variant="inset" collapsible="offcanvas">
        <SidebarHeader className="h-12 flex-row items-center">
          <SidebarTrigger className="text-muted-foreground" />
        </SidebarHeader>

        <SidebarContent>
          <SidebarGroup className="mt-4">
            <SidebarMenu>
              <NavItem active={isActive(scope, { kind: 'all' })} icon={<Inbox />} label="All articles" onClick={() => onNavigate({ kind: 'all' })} />
              <NavItem
                active={isActive(scope, { kind: 'unread' })}
                icon={<CircleDot />}
                label="Unread"
                count={sidebar?.total_unread}
                onClick={() => onNavigate({ kind: 'unread' })}
              />
              <NavItem active={isActive(scope, { kind: 'starred' })} icon={<Star />} label="Starred" onClick={() => onNavigate({ kind: 'starred' })} />
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
                <DropdownMenuItem onSelect={() => onOpenDialog('add')}>Add feed</DropdownMenuItem>
                <DropdownMenuItem onSelect={() => setCreatingFolder(true)}>New folder</DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>

            <SidebarGroupContent>
              <SidebarMenu>
                {creatingFolder ? (
                  <SidebarMenuItem>
                    <RenameInput
                      initial=""
                      label="Folder name"
                      onDone={(name) => {
                        if (name) actions.createFolder(name)
                        setCreatingFolder(false)
                      }}
                    />
                  </SidebarMenuItem>
                ) : null}

                {folders.map((folder) => (
                  // Feed rows are siblings of their folder row, not children, so hover and menu
                  // state on one row never leaks into another through `group` variants.
                  <FolderRows
                    key={folder.id}
                    folder={folder}
                    open={!collapsed.includes(folder.id)}
                    active={isActive(scope, { kind: 'folder', id: folder.id })}
                    renaming={renaming === `folder:${folder.id}`}
                    dropBefore={drag.dropsBefore(`folder:${folder.id}`)}
                    dropInto={drag.dropsInto(`folder:${folder.id}`)}
                    dragProps={drag.folder(folder)}
                    onOpen={() => onNavigate({ kind: 'folder', id: folder.id })}
                    onToggle={() => toggleFolder(folder.id)}
                    onRename={() => setRenaming(`folder:${folder.id}`)}
                    onRenamed={(name) => {
                      if (name) actions.renameFolder({ id: folder.id, name })
                      setRenaming(null)
                    }}
                    onRemove={() => setRemoval({ kind: 'folder', folder })}
                  >
                    {folder.feeds.map((feed) => feedRow(feed, folder.id, folder.feeds))}
                  </FolderRows>
                ))}

                {uncategorized.map((feed) => feedRow(feed, null, uncategorized))}

                {drag.draggingFeed ? (
                  <li
                    className={cn(
                      'mt-2 flex items-center gap-2 rounded-md border border-dashed p-2.5 text-xs text-muted-foreground',
                      drag.overOutOfFolder ? 'border-signal bg-signal/8' : 'border-faint',
                    )}
                    {...drag.outOfFolder}
                  >
                    <FolderInput className="size-3.5" /> Drop here to take it out of its folder
                  </li>
                ) : null}

                {sidebar && folders.length === 0 && uncategorized.length === 0 && !creatingFolder ? (
                  <SidebarMenuItem>
                    <SidebarMenuButton onClick={() => onOpenDialog('add')} className="text-muted-foreground">
                      <Plus />
                      Add your first feed
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                ) : null}
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>
        </SidebarContent>

        <SidebarFooter>
          <SidebarMenu>
            <NavItem icon={<FolderInput />} label="Import & export" onClick={() => onOpenDialog('transfer')} />
            <NavItem icon={<Keyboard />} label="Keyboard shortcuts" onClick={() => onOpenDialog('shortcuts')} />
          </SidebarMenu>
        </SidebarFooter>
      </Sidebar>

      <RemoveDialog removal={removal} onCancel={() => setRemoval(null)} onConfirm={remove} />
    </>
  )
})

type NavItemProps = {
  icon: ReactNode
  label: string
  onClick: () => void
  active?: boolean
  count?: number
}

function NavItem({ icon, label, onClick, active = false, count }: NavItemProps) {
  return (
    <SidebarMenuItem>
      <SidebarMenuButton isActive={active} onClick={onClick}>
        {icon}
        {label}
      </SidebarMenuButton>
      <UnreadCount value={count} />
    </SidebarMenuItem>
  )
}

type DragProps = ReturnType<ReturnType<typeof useTreeDrag>['feed']>

type FeedRowProps = {
  feed: SidebarFeed
  nested: boolean
  active: boolean
  renaming: boolean
  dropBefore: boolean
  dragProps: DragProps
  onOpen: () => void
  onRename: () => void
  onRenamed: (title: string | undefined) => void
  onRemove: () => void
}

function FeedRow({ feed, nested, active, renaming, dropBefore, dragProps, onOpen, onRename, onRenamed, onRemove }: FeedRowProps) {
  return (
    <SidebarMenuItem className={cn(nested && 'pl-4', dropBefore && dropLine)} draggable={!renaming} {...dragProps}>
      {renaming ? (
        <RenameInput initial={feed.title} label="Feed name" onDone={onRenamed} />
      ) : (
        <>
          <SidebarMenuButton isActive={active} onClick={onOpen} className="text-muted-foreground hover:text-foreground data-[active=true]:text-foreground">
            <FeedIcon siteUrl={feed.site_url} />
            <span className="truncate">{feed.title}</span>
            {feed.last_error ? <AlertCircle className="text-warning" aria-label={`Not updating: ${feed.last_error}`} /> : null}
          </SidebarMenuButton>
          <UnreadCount value={feed.unread} />
          <RowMenu
            label={feed.title}
            onRename={onRename}
            onRefresh={() => api.refresh({ kind: 'feed', id: feed.id })}
            destructiveLabel="Unsubscribe…"
            onDestroy={onRemove}
          />
        </>
      )}
    </SidebarMenuItem>
  )
}

type FolderRowsProps = {
  folder: SidebarFolder
  open: boolean
  active: boolean
  renaming: boolean
  dropBefore: boolean
  dropInto: boolean
  dragProps: DragProps
  onOpen: () => void
  onToggle: () => void
  onRename: () => void
  onRenamed: (name: string | undefined) => void
  onRemove: () => void
  /** The folder's feed rows, shown while it's open. */
  children: ReactNode
}

function FolderRows({ folder, open, active, renaming, dropBefore, dropInto, dragProps, onOpen, onToggle, onRename, onRenamed, onRemove, children }: FolderRowsProps) {
  return (
    <>
      <SidebarMenuItem className={cn(dropBefore && dropLine)} draggable={!renaming} {...dragProps}>
        {renaming ? (
          <RenameInput initial={folder.name} label="Folder name" onDone={onRenamed} />
        ) : (
          <>
            <SidebarMenuButton isActive={active} onClick={onOpen} className={cn('pl-8', dropInto && 'bg-signal/12')}>
              <span className="truncate">{folder.name}</span>
            </SidebarMenuButton>
            <button
              type="button"
              aria-expanded={open}
              aria-label={open ? `Collapse ${folder.name}` : `Expand ${folder.name}`}
              onClick={onToggle}
              className="absolute top-1.5 left-1.5 grid size-5 place-items-center rounded-sm text-faint hover:text-foreground"
            >
              <ChevronRight className={cn('size-3.5 transition-transform motion-reduce:transition-none', open && 'rotate-90')} />
            </button>
            <UnreadCount value={folder.unread} />
            <RowMenu
              label={folder.name}
              onRename={onRename}
              onRefresh={() => api.refresh({ kind: 'folder', id: folder.id })}
              destructiveLabel="Delete folder…"
              onDestroy={onRemove}
            />
          </>
        )}
      </SidebarMenuItem>
      {open ? children : null}
    </>
  )
}
