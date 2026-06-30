/** Return a new array with the element at `from` moved to index `to`. Input is not mutated. */
export function moveItem<T>(arr: T[], from: number, to: number): T[] {
  const next = [...arr];
  const [moved] = next.splice(from, 1);
  next.splice(to, 0, moved);
  return next;
}

/**
 * Reorder `items` to follow the id sequence in `idOrder`.
 * Ids with no matching item are skipped; items whose id is absent from
 * `idOrder` are dropped. Used to restore the pre-reorder snapshot on cancel.
 */
export function restoreOrder<T extends { id: string }>(items: T[], idOrder: string[]): T[] {
  const byId = new Map(items.map((it) => [it.id, it]));
  return idOrder
    .map((id) => byId.get(id))
    .filter((it): it is T => it !== undefined);
}
