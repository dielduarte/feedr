import { ArrowLeft, ArrowUpRight, CheckCheck, ChevronDown, Circle, CircleCheck, CircleDot, Folder, Inbox, RefreshCw, Star, WifiOff } from 'lucide-react'
import { useEffect, useState } from 'react'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Separator } from '@/components/ui/separator'
import { SidebarTrigger, useSidebar } from '@/components/ui/sidebar'
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group'
import { cn } from '@/lib/utils'
import type { Item, Sidebar } from '../api'
import type { PollerStatus } from '../queries'
import { scopePath, type Scope } from '../routes'
import { FeedIcon } from './FeedIcon'
import { IconAction } from './IconAction'

type ListActions = {
  unreadOnly: boolean
  canFilterUnread: boolean
  onUnreadOnlyChange: (unreadOnly: boolean) => void
  canMarkAllRead: boolean
  onMarkAllRead: () => void
}

type ReaderActions = {
  item: Item | undefined
  words: number
  onBack: () => void
  onToggleStar: () => void
  onToggleRead: () => void
}

type Props = {
  scope: Scope
  scopeLabel: string
  sidebar: Sidebar | undefined
  onNavigate: (scope: Scope) => void
  status: PollerStatus
  onRefresh: () => void
  view: { kind: 'list'; actions: ListActions } | { kind: 'reader'; actions: ReaderActions }
}

export function TopBar({ scope, scopeLabel, sidebar, onNavigate, status, onRefresh, view }: Props) {
  const { state, isMobile } = useSidebar()
  const sidebarHidden = state === 'collapsed' || isMobile

  return (
    <header className="flex h-13 shrink-0 items-center justify-between gap-3 border-b px-3">
      <div className="flex min-w-0 items-center gap-2">
        {/* Exactly one icon ever sits before the switcher, so it never shifts or leaves a gap. */}
        {view.kind === 'reader' ? (
          <IconAction label="Back to articles" shortcut="Esc" onClick={view.actions.onBack}>
            <ArrowLeft />
          </IconAction>
        ) : sidebarHidden ? (
          <SidebarTrigger className="size-8 text-muted-foreground" />
        ) : (
          <span className="grid size-8 shrink-0 place-items-center text-muted-foreground [&_svg]:size-4" aria-hidden>
            <ScopeIcon scope={scope} sidebar={sidebar} />
          </span>
        )}
        <ScopeSwitcher scope={scope} label={scopeLabel} sidebar={sidebar} onNavigate={onNavigate} />
        {view.kind === 'reader' && view.actions.item?.title && (
          <span className="truncate text-[13.5px] text-muted-foreground max-md:hidden">{view.actions.item.title}</span>
        )}
      </div>

      <div className="flex items-center gap-1">
        {view.kind === 'list' ? <ListTools {...view.actions} /> : <ReaderTools {...view.actions} />}
        {status.offline && (
          <span className="flex items-center gap-1.5 px-2 text-xs text-muted-foreground" title="feedrsauros can't reach the internet and will retry.">
            <WifiOff className="size-3.5" /> Offline
          </span>
        )}
        <IconAction label={status.refreshing ? 'Refreshing feeds' : 'Refresh feeds'} shortcut="R" onClick={onRefresh}>
          <RefreshIcon refreshing={status.refreshing} />
        </IconAction>
      </div>
    </header>
  )
}

function ListTools({ unreadOnly, canFilterUnread, onUnreadOnlyChange, canMarkAllRead, onMarkAllRead }: ListActions) {
  return (
    <>
      {canFilterUnread && (
        <ToggleGroup
          type="single"
          size="sm"
          value={unreadOnly ? 'unread' : 'all'}
          onValueChange={(value) => value && onUnreadOnlyChange(value === 'unread')}
          className="mr-1 rounded-lg bg-secondary p-0.5"
          aria-label="Show"
        >
          <ToggleGroupItem value="all" className="h-7 rounded-md px-2.5 text-xs data-[state=on]:bg-background data-[state=on]:shadow-xs">
            All
          </ToggleGroupItem>
          <ToggleGroupItem value="unread" className="h-7 rounded-md px-2.5 text-xs data-[state=on]:bg-background data-[state=on]:shadow-xs">
            Unread
          </ToggleGroupItem>
        </ToggleGroup>
      )}
      <IconAction label="Mark all as read" shortcut="⇧A" onClick={onMarkAllRead} disabled={!canMarkAllRead}>
        <CheckCheck />
      </IconAction>
    </>
  )
}

function ReaderTools({ item, words, onToggleStar, onToggleRead }: ReaderActions) {
  if (!item) return null
  return (
    <>
      {words > 0 && (
        <span className="px-2 text-[13px] text-muted-foreground tabular-nums max-md:hidden">
          {words.toLocaleString('en-US')} words
        </span>
      )}
      <Separator orientation="vertical" className="mr-1 data-[orientation=vertical]:h-4 max-md:hidden" />
      <IconAction label={item.starred_at ? 'Unstar' : 'Star'} shortcut="S" pressed={item.starred_at !== null} onClick={onToggleStar}>
        <Star />
      </IconAction>
      <IconAction label={item.read_at ? 'Mark as unread' : 'Mark as read'} shortcut="M" onClick={onToggleRead}>
        {item.read_at ? <Circle /> : <CircleCheck />}
      </IconAction>
      {item.url && (
        <IconAction label="Open original" shortcut="V" asChild>
          <a href={item.url} target="_blank" rel="noopener noreferrer">
            <ArrowUpRight />
          </a>
        </IconAction>
      )}
    </>
  )
}

function ScopeIcon({ scope, sidebar }: { scope: Scope; sidebar: Sidebar | undefined }) {
  switch (scope.kind) {
    case 'all':
      return <Inbox />
    case 'unread':
      return <CircleDot />
    case 'starred':
      return <Star />
    case 'folder':
      return <Folder />
    case 'feed': {
      const feeds = [...(sidebar?.uncategorized ?? []), ...(sidebar?.folders.flatMap((f) => f.feeds) ?? [])]
      return <FeedIcon siteUrl={feeds.find((f) => f.id === scope.id)?.site_url ?? null} />
    }
  }
}

type SwitcherProps = {
  scope: Scope
  label: string
  sidebar: Sidebar | undefined
  onNavigate: (scope: Scope) => void
}

function ScopeSwitcher({ scope, label, sidebar, onNavigate }: SwitcherProps) {
  const current = scopePath(scope)
  const option = (target: Scope, text: string, nested = false) => (
    <DropdownMenuItem
      key={scopePath(target)}
      onSelect={() => onNavigate(target)}
      className={cn(nested && 'pl-5 text-muted-foreground', scopePath(target) === current && 'font-semibold text-foreground')}
    >
      {text}
    </DropdownMenuItem>
  )
  const hasFeeds = !!sidebar && (sidebar.folders.length > 0 || sidebar.uncategorized.length > 0)

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="secondary" size="sm" className="h-7.5 gap-1.5 px-2.5 text-[13.5px] font-normal">
          {label}
          <ChevronDown className="size-3.5" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="dark max-h-[60vh] min-w-60 overflow-y-auto">
        {option({ kind: 'all' }, 'All articles')}
        {option({ kind: 'unread' }, 'Unread')}
        {option({ kind: 'starred' }, 'Starred')}
        {hasFeeds && <DropdownMenuSeparator />}
        {sidebar?.folders.map((folder) => [
          option({ kind: 'folder', id: folder.id }, folder.name),
          ...folder.feeds.map((feed) => option({ kind: 'feed', id: feed.id }, feed.title, true)),
        ])}
        {sidebar?.uncategorized.map((feed) => option({ kind: 'feed', id: feed.id }, feed.title))}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

/**
 * Spins while refreshing and always finishes the turn it's on, so even an instant refresh shows
 * one full rotation and the icon never stops at an odd angle.
 */
function RefreshIcon({ refreshing }: { refreshing: boolean }) {
  const [spinning, setSpinning] = useState(refreshing)

  useEffect(() => {
    if (refreshing) setSpinning(true)
  }, [refreshing])

  return (
    <RefreshCw
      className={cn(spinning && 'animate-spin motion-reduce:[animation-duration:3s]')}
      onAnimationIteration={() => !refreshing && setSpinning(false)}
    />
  )
}
