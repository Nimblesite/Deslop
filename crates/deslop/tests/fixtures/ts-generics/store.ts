export interface Repository<TEntity, TIndex> {
  fetch(id: TIndex): Promise<TEntity>;
}

export async function readEvery<TEntity, TIndex>(
  repository: Repository<TEntity, TIndex>,
  identifiers: ReadonlyArray<TIndex>,
): Promise<Map<TIndex, TEntity>> {
  const resolved = new Map<TIndex, TEntity>();
  for (const identifier of identifiers) {
    const entity = await repository.fetch(identifier);
    resolved.set(identifier, entity);
  }
  return resolved;
}
