export type Scope =
  | { kind: 'all' }
  | { kind: 'unread' }
  | { kind: 'starred' }
  | { kind: 'folder'; slug: string }
  | { kind: 'feed'; slug: string }

/** An article is addressed by its feed's slug and its own, which is unique within the feed. */
export type ArticleRef = { feed: string; slug: string }

export type Location = { scope: Scope; article: ArticleRef | null }

export function scopePath(scope: Scope): string {
  switch (scope.kind) {
    case 'all':
      return '/'
    case 'unread':
      return '/unread'
    case 'starred':
      return '/starred'
    case 'folder':
      return `/folders/${encodeURIComponent(scope.slug)}`
    case 'feed':
      return `/feeds/${encodeURIComponent(scope.slug)}`
  }
}

export function articlePath(article: ArticleRef): string {
  return `/feeds/${encodeURIComponent(article.feed)}/${encodeURIComponent(article.slug)}`
}

export function articleKey(article: ArticleRef): string {
  return `${article.feed}/${article.slug}`
}

const ALL: Location = { scope: { kind: 'all' }, article: null }

export function parseLocation(path: string): Location {
  const segments = path.split('/').filter(Boolean).map(decodeURIComponent)
  const [section, slug, article, ...rest] = segments
  if (rest.length > 0) return ALL

  if ((section === 'unread' || section === 'starred') && slug === undefined) {
    return { scope: { kind: section }, article: null }
  }
  if (section === 'folders' && slug !== undefined && article === undefined) {
    return { scope: { kind: 'folder', slug }, article: null }
  }
  if (section === 'feeds' && slug !== undefined) {
    return {
      scope: { kind: 'feed', slug },
      article: article === undefined ? null : { feed: slug, slug: article },
    }
  }
  return ALL
}
