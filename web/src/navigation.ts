/** The article `step` places from `current`, clamped to the ends; the first one when none is selected. */
export function moveSelection<K>(keys: readonly K[], current: K | null, step: number): K | null {
  if (keys.length === 0) return null
  const index = current === null ? -1 : keys.indexOf(current)
  if (index === -1) return keys[0]
  return keys[Math.min(keys.length - 1, Math.max(0, index + step))]
}

/** The article next to the open one, or null past either end or when it isn't in the list. */
export function neighbour<K>(keys: readonly K[], current: K, step: number): K | null {
  const index = keys.indexOf(current)
  if (index === -1) return null
  return keys[index + step] ?? null
}

/** Whether `key` is among the last `threshold` loaded articles, so the next page should load. */
export function nearEnd<K>(keys: readonly K[], key: K, threshold: number): boolean {
  const index = keys.indexOf(key)
  return index !== -1 && index >= keys.length - threshold
}
