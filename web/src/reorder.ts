/**
 * Where to place `dragged` among `siblings` (the destination's current order) so it lands just
 * before `target`, or last. The server removes the dragged item before inserting it, so indexes
 * are counted without it.
 */
export function dropIndex<K>(siblings: readonly K[], dragged: K, target: K | 'end'): number {
  const remaining = siblings.filter((id) => id !== dragged)
  const index = target === 'end' ? -1 : remaining.indexOf(target)
  return index === -1 ? remaining.length : index
}
