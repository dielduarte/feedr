import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { api, type ImportReport } from '../../api'
import { sentence } from '../../format'
import { useSidebarMutation } from '../../queries'
import { panel } from './panel'

function describe(report: ImportReport): string {
  const plural = (count: number, one: string, many: string) => `${count} ${count === 1 ? one : many}`
  const parts = [`Added ${plural(report.added, 'feed', 'feeds')}`]
  if (report.skipped > 0) parts.push(`skipped ${report.skipped} already subscribed`)
  if (report.invalid.length > 0) parts.push(`ignored ${plural(report.invalid.length, 'invalid address', 'invalid addresses')}`)
  return `${parts.join(', ')}.${report.added > 0 ? ' They are being fetched now.' : ''}`
}

export function TransferDialog({ onClose }: { onClose: () => void }) {
  const importFile = useSidebarMutation((file: File) => api.importOpml(file), { inlineErrors: true })

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent className={panel}>
        <DialogHeader>
          <DialogTitle>Import & export</DialogTitle>
          <DialogDescription>Move your subscriptions between feedrsauros and other readers with an OPML file.</DialogDescription>
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
                  if (file) importFile.mutate(file)
                  event.target.value = ''
                }}
              />
            </label>
          </Button>
          <Button variant="secondary" asChild>
            <a href="/api/opml" download="feedrsauros.opml">
              Export OPML file
            </a>
          </Button>
        </div>

        {importFile.data ? <p className="text-sm text-muted-foreground">{describe(importFile.data)}</p> : null}
        {importFile.error ? <p className="text-sm text-destructive">{sentence(importFile.error.message)}</p> : null}
      </DialogContent>
    </Dialog>
  )
}
