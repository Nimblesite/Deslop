export interface Repository<TEntity, TKey> {
  fetch(id: TKey): Promise<TEntity>;
}

export async function loadAll<TEntity, TKey>(
  repository: Repository<TEntity, TKey>,
  identifiers: ReadonlyArray<TKey>,
): Promise<Map<TKey, TEntity>> {
  const resolved = new Map<TKey, TEntity>();
  for (const identifier of identifiers) {
    const entity = await repository.fetch(identifier);
    resolved.set(identifier, entity);
  }
  return resolved;
}
