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
import type { SidebarFeed, SidebarFolder } from '../../api'

export type Removal = { kind: 'feed'; feed: SidebarFeed } | { kind: 'folder'; folder: SidebarFolder }

type Props = {
  removal: Removal | null
  onCancel: () => void
  onConfirm: (removal: Removal) => void
}

export function RemoveDialog({ removal, onCancel, onConfirm }: Props) {
  const copy =
    removal?.kind === 'feed'
      ? {
          title: `Unsubscribe from ${removal.feed.title}?`,
          description: 'Its articles are removed too, including starred ones.',
          action: 'Unsubscribe',
        }
      : removal?.kind === 'folder'
        ? {
            title: `Delete ${removal.folder.name}?`,
            description: 'Its feeds stay subscribed, outside any folder.',
            action: 'Delete folder',
          }
        : null

  return (
    <AlertDialog open={removal !== null} onOpenChange={(open) => !open && onCancel()}>
      <AlertDialogContent className="dark bg-popover text-popover-foreground sm:max-w-sm">
        {copy ? (
          <AlertDialogHeader>
            <AlertDialogTitle>{copy.title}</AlertDialogTitle>
            <AlertDialogDescription>{copy.description}</AlertDialogDescription>
          </AlertDialogHeader>
        ) : null}
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            className="bg-destructive text-background hover:bg-destructive/90"
            onClick={() => removal && onConfirm(removal)}
          >
            {copy?.action}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}
