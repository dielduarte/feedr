/** The article `step` places from `current`, clamped to the ends; the first one when none is selected. */
export function moveSelection(ids: readonly number[], current: number | null, step: number): number | null {
  if (ids.length === 0) return null
  const index = current === null ? -1 : ids.indexOf(current)
  if (index === -1) return ids[0]
  return ids[Math.min(ids.length - 1, Math.max(0, index + step))]
}

/** The article next to the open one, or null past either end or when it isn't in the list. */
export function neighbour(ids: readonly number[], current: number, step: number): number | null {
  const index = ids.indexOf(current)
  if (index === -1) return null
  return ids[index + step] ?? null
}

/** Whether `id` is among the last `threshold` loaded articles, so the next page should load. */
export function nearEnd(ids: readonly number[], id: number, threshold: number): boolean {
  const index = ids.indexOf(id)
  return index !== -1 && index >= ids.length - threshold
}
