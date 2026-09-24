import { type FormEvent, useState } from 'react'
import { Button } from '@/components/ui/button'
import { Dialog, DialogClose, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { api, type Sidebar } from '../../api'
import { sentence } from '../../format'
import { useSidebarMutation } from '../../queries'
import { panel } from './panel'

const NO_FOLDER = 'none'

type Props = {
  sidebar: Sidebar | undefined
  defaultFolder: number | null
  onClose: () => void
  onAdded: (feedId: number) => void
}

export function AddFeedDialog({ sidebar, defaultFolder, onClose, onAdded }: Props) {
  const [url, setUrl] = useState('')
  const [folder, setFolder] = useState(defaultFolder === null ? NO_FOLDER : String(defaultFolder))
  const subscribe = useSidebarMutation(
    (args: { url: string; folder: number | null }) => api.subscribe(args.url, args.folder),
    { inlineErrors: true },
  )
  const address = url.trim()

  const submit = (event: FormEvent) => {
    event.preventDefault()
    if (!address) return
    subscribe.mutate(
      { url: address, folder: folder === NO_FOLDER ? null : Number(folder) },
      { onSuccess: (added) => onAdded(added.id) },
    )
  }

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent className={panel}>
        <form onSubmit={submit} className="grid gap-4">
          <DialogHeader>
            <DialogTitle>Add a feed</DialogTitle>
            <DialogDescription>Paste a website and feedrsauros finds its feed.</DialogDescription>
          </DialogHeader>

          <Input
            autoFocus
            autoComplete="off"
            data-1p-ignore
            data-lpignore="true"
            aria-label="Site or feed address"
            placeholder="example.com"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
          />

          {sidebar && sidebar.folders.length > 0 ? (
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
          ) : null}

          {subscribe.error ? <p className="text-sm text-destructive">{sentence(subscribe.error.message)}</p> : null}

          <DialogFooter className="grid grid-cols-2 gap-2 sm:grid-cols-2">
            <DialogClose asChild>
              <Button type="button" variant="secondary">
                Cancel
              </Button>
            </DialogClose>
            <Button type="submit" disabled={subscribe.isPending || !address}>
              {subscribe.isPending ? 'Adding…' : 'Add feed'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
