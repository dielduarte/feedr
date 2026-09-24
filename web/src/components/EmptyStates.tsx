import { Button } from '@/components/ui/button'
import type { Scope } from '../routes'

export function EmptyList({ scope, unreadOnly }: { scope: Scope; unreadOnly: boolean }) {
  const [title, text] =
    scope.kind === 'starred'
      ? ['Nothing starred yet', 'Press S on an article to keep it here.']
      : unreadOnly
        ? ["You're all caught up", 'New articles show up here as your feeds update.']
        : ['No articles yet', "feedrsauros checks your feeds regularly; articles appear as they're published."]
  return (
    <div className="py-14">
      <p className="mb-1.5 text-[17px] font-medium">{title}</p>
      <p className="max-w-[46ch] leading-relaxed text-muted-foreground">{text}</p>
    </div>
  )
}

export function Welcome({ onAdd, onImport }: { onAdd: () => void; onImport: () => void }) {
  return (
    <div className="py-14">
      <p className="mb-2 text-2xl font-semibold tracking-[-0.02em]">Start with a site you read</p>
      <p className="max-w-[46ch] leading-relaxed text-muted-foreground">
        Paste its address and feedrsauros finds the feed, then keeps it up to date while it runs.
      </p>
      <div className="mt-5.5 flex items-center gap-2">
        <Button onClick={onAdd}>Add a feed</Button>
        <Button variant="ghost" onClick={onImport}>
          Import from another reader
        </Button>
      </div>
    </div>
  )
}
