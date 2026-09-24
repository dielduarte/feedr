export type Scope =
  | { kind: 'all' }
  | { kind: 'unread' }
  | { kind: 'starred' }
  | { kind: 'folder'; id: number }
  | { kind: 'feed'; id: number }

export type Location = { scope: Scope; itemId: number | null }

export function scopePath(scope: Scope): string {
  switch (scope.kind) {
    case 'all':
      return '/'
    case 'unread':
      return '/unread'
    case 'starred':
      return '/starred'
    case 'folder':
      return `/folders/${scope.id}`
    case 'feed':
      return `/feeds/${scope.id}`
  }
}

export function itemPath(scope: Scope, itemId: number): string {
  const base = scopePath(scope)
  return `${base === '/' ? '' : base}/items/${itemId}`
}

const ALL: Scope = { kind: 'all' }

function id(segment: string | undefined): number | null {
  return segment !== undefined && /^\d+$/.test(segment) ? Number(segment) : null
}

export function parseLocation(path: string): Location {
  const segments = path.split('/').filter(Boolean)
  let scope: Scope = ALL
  let rest = segments

  const [first, second] = segments
  if (first === 'unread' || first === 'starred') {
    scope = { kind: first }
    rest = segments.slice(1)
  } else if ((first === 'folders' || first === 'feeds') && id(second) !== null) {
    scope = { kind: first === 'folders' ? 'folder' : 'feed', id: id(second)! }
    rest = segments.slice(2)
  }

  const itemId = rest[0] === 'items' ? id(rest[1]) : null
  return { scope, itemId }
}
