import type { Sidebar } from './api'
import type { Shortcuts } from './lib/hooks'
import type { Lookup } from './lookup'
import type { PollerStatus } from './poller'
import type { Scope } from './routes'

/** What every page needs from the shell to draw its top bar and share global shortcuts. */
export type Chrome = {
  scope: Scope
  label: string
  sidebar: Sidebar | undefined
  lookup: Lookup
  status: PollerStatus
  onNavigate: (scope: Scope) => void
  onRefresh: () => void
  /** Shortcuts that work on every page. */
  shortcuts: Shortcuts
  shortcutsEnabled: boolean
}
