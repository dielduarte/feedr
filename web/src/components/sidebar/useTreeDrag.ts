import { type DragEvent, useState } from 'react'
import type { SidebarFeed, SidebarFolder } from '../../api'
import { dropIndex } from '../../reorder'

type Dragging = { kind: 'feed'; id: number } | { kind: 'folder'; id: number } | null

type Options = {
  folders: SidebarFolder[]
  uncategorized: SidebarFeed[]
  onMoveFeed: (move: { id: number; folderId: number | null; index: number }) => void
  onMoveFolder: (move: { id: number; index: number }) => void
}

/** Drag-and-drop for the subscription tree: feeds into folders or between feeds, folders among folders. */
export function useTreeDrag({ folders, uncategorized, onMoveFeed, onMoveFolder }: Options) {
  const [dragging, setDragging] = useState<Dragging>(null)
  const [over, setOver] = useState<string | null>(null)

  const end = () => {
    setDragging(null)
    setOver(null)
  }

  const source = (item: Exclude<Dragging, null>, name: string) => ({
    onDragStart: (event: DragEvent) => {
      event.dataTransfer.effectAllowed = 'move'
      event.dataTransfer.setData('text/plain', name)
      setDragging(item)
    },
    onDragEnd: end,
  })

  const target = (key: string, accepts: boolean, drop: () => void) => ({
    onDragOver: (event: DragEvent) => {
      if (!accepts) return
      event.preventDefault()
      event.stopPropagation()
      setOver(key)
    },
    onDragLeave: () => setOver((current) => (current === key ? null : current)),
    onDrop: (event: DragEvent) => {
      event.preventDefault()
      event.stopPropagation()
      drop()
      end()
    },
  })

  const dropFeed = (folderId: number | null, siblings: SidebarFeed[], before: number | 'end') => {
    if (dragging?.kind !== 'feed') return
    onMoveFeed({ id: dragging.id, folderId, index: dropIndex(siblings.map((f) => f.id), dragging.id, before) })
  }

  return {
    draggingFeed: dragging?.kind === 'feed',
    /** A line above this row: something will be dropped before it. */
    dropsBefore: (key: string) =>
      over === key && (key.startsWith('feed:') ? dragging?.kind === 'feed' : dragging?.kind === 'folder'),
    /** The folder row is highlighted: a feed will be dropped into it. */
    dropsInto: (key: string) => over === key && key.startsWith('folder:') && dragging?.kind === 'feed',
    overOutOfFolder: over === 'uncategorized',

    feed: (feed: SidebarFeed, folderId: number | null, siblings: SidebarFeed[]) => ({
      ...source({ kind: 'feed', id: feed.id }, feed.title),
      ...target(`feed:${feed.id}`, dragging?.kind === 'feed' && dragging.id !== feed.id, () =>
        dropFeed(folderId, siblings, feed.id),
      ),
    }),

    folder: (folder: SidebarFolder) => ({
      ...source({ kind: 'folder', id: folder.id }, folder.name),
      ...target(
        `folder:${folder.id}`,
        dragging !== null && !(dragging.kind === 'folder' && dragging.id === folder.id),
        () => {
          if (dragging?.kind === 'feed') dropFeed(folder.id, folder.feeds, 'end')
          if (dragging?.kind === 'folder') {
            onMoveFolder({ id: dragging.id, index: dropIndex(folders.map((f) => f.id), dragging.id, folder.id) })
          }
        },
      ),
    }),

    outOfFolder: target('uncategorized', true, () => dropFeed(null, uncategorized, 'end')),
  }
}
