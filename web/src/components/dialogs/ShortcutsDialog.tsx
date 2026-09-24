import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Kbd } from '@/components/ui/kbd'
import { panel } from './panel'

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

export function ShortcutsDialog({ onClose }: { onClose: () => void }) {
  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
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
