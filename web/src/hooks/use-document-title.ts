import { useEffect } from 'react'

export function useDocumentTitle(title: string, unread: number) {
  useEffect(() => {
    document.title = `${unread > 0 ? `(${unread}) ` : ''}${title} — feedrsauros`
  }, [title, unread])
}
