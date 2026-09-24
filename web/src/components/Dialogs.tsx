import { type FormEvent, useState } from 'react'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Kbd } from '@/components/ui/kbd'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { api, type ImportReport, type Sidebar } from '../api'
import { useSidebarMutation } from '../queries'

const panel = 'dark bg-popover text-popover-foreground sm:max-w-sm'
const NO_FOLDER = 'none'

type Closable = { open: boolean; onOpenChange: (open: boolean) => void }

type AddFeedProps = Closable & {
  sidebar: Sidebar | undefined
  defaultFolder: number | null
  onAdded: (feedId: number) => void
}

export function AddFeedDialog({ open, onOpenChange, sidebar, defaultFolder, onAdded }: AddFeedProps) {
  const [url, setUrl] = useState('')
  const [folder, setFolder] = useState(defaultFolder === null ? NO_FOLDER : String(defaultFolder))
  const subscribe = useSidebarMutation(
    (args: { url: string; folder: number | null }) => api.subscribe(args.url, args.folder),
    { inlineErrors: true },
  )

  const submit = (event: FormEvent) => {
    event.preventDefault()
    if (!url.trim()) return
    subscribe.mutate(
      { url: url.trim(), folder: folder === NO_FOLDER ? null : Number(folder) },
      {
        onSuccess: (added) => {
          setUrl('')
          onAdded(added.id)
        },
      },
    )
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className={panel}>
        <form onSubmit={submit} className="grid gap-4">
          <DialogHeader>
            <DialogTitle>Add a feed</DialogTitle>
            <DialogDescription>Paste a website and feedr finds its feed.</DialogDescription>
          </DialogHeader>

          <Input
            autoFocus
            autoComplete="off"
            data-1p-ignore
            data-lpignore="true"
            aria-label="Site or feed address"
            placeholder="example.com" value={url} onChange={(e) => setUrl(e.target.value)} />

          {sidebar && sidebar.folders.length > 0 && (
            <Select value={folder} onValueChange={setFolder}>
              <SelectTrigger aria-label="Folder" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent className="dark">
                <SelectItem value={NO_FOLDER}>No folder</SelectItem>
                {sidebar.folders.map((f) => (
                  <SelectItem key={f.id} value={String(f.id)}>
                    {f.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          )}

          {subscribe.error && <p className="text-sm text-destructive">{sentence(subscribe.error.message)}</p>}

          <DialogFooter className="grid grid-cols-2 gap-2 sm:grid-cols-2">
            <DialogClose asChild>
              <Button type="button" variant="secondary">Cancel</Button>
            </DialogClose>
            <Button type="submit" disabled={subscribe.isPending || !url.trim()}>
              {subscribe.isPending ? 'Adding…' : 'Add feed'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}

export function TransferDialog({ open, onOpenChange }: Closable) {
  const [report, setReport] = useState<ImportReport | null>(null)
  const importFile = useSidebarMutation((file: File) => api.importOpml(file), { inlineErrors: true })

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className={panel}>
        <DialogHeader>
          <DialogTitle>Import & export</DialogTitle>
          <DialogDescription>Move your subscriptions between feedr and other readers with an OPML file.</DialogDescription>
        </DialogHeader>

        <div className="grid gap-2">
          <Button variant="secondary" className="relative overflow-hidden focus-within:ring-2 focus-within:ring-ring" asChild>
            <label>
              {importFile.isPending ? 'Importing…' : 'Import OPML file'}
              <input
                type="file"
                accept=".opml,.xml,text/x-opml,text/xml"
                className="absolute inset-0 cursor-pointer opacity-0"
                onChange={(event) => {
                  const file = event.target.files?.[0]
                  if (file) importFile.mutate(file, { onSuccess: setReport })
                  event.target.value = ''
                }}
              />
            </label>
          </Button>
          <Button variant="secondary" asChild>
            <a href="/api/opml" download="feedr.opml">Export OPML file</a>
          </Button>
        </div>

        {report && <p className="text-sm text-muted-foreground">{describe(report)}</p>}
        {importFile.error && <p className="text-sm text-destructive">{sentence(importFile.error.message)}</p>}
      </DialogContent>
    </Dialog>
  )
}

const SHORTCUTS: [string[], string][] = [
  [['J', 'K'], 'Next / previous article'],
  [['Enter'], 'Open article'],
  [['Esc'], 'Back to the list'],
  [['S'], 'Star or unstar'],
  [['M'], 'Mark read or unread'],
  [['V'], 'Open the original page'],
  [['⇧', 'A'], 'Mark all as read'],
  [['R'], 'Refresh feeds'],
  [['['], 'Show or hide the sidebar'],
]

export function ShortcutsDialog({ open, onOpenChange }: Closable) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className={panel}>
        <DialogHeader>
          <DialogTitle>Keyboard shortcuts</DialogTitle>
        </DialogHeader>
        <dl className="grid gap-2.5 text-sm">
          {SHORTCUTS.map(([keys, action]) => (
            <div key={action} className="flex items-center justify-between gap-4">
              <dd className="text-muted-foreground">{action}</dd>
              <dt className="flex gap-1">
                {keys.map((key) => (
                  <Kbd key={key}>{key}</Kbd>
                ))}
              </dt>
            </div>
          ))}
        </dl>
      </DialogContent>
    </Dialog>
  )
}

function describe(report: ImportReport): string {
  const parts = [`Added ${plural(report.added, 'feed')}`]
  if (report.skipped > 0) parts.push(`skipped ${report.skipped} already subscribed`)
  if (report.invalid.length > 0) parts.push(`ignored ${plural(report.invalid.length, 'invalid address', 'invalid addresses')}`)
  return `${parts.join(', ')}.${report.added > 0 ? ' They are being fetched now.' : ''}`
}

function plural(count: number, word: string, many = `${word}s`) {
  return `${count} ${count === 1 ? word : many}`
}

function sentence(text: string) {
  return text.charAt(0).toUpperCase() + text.slice(1)
}
