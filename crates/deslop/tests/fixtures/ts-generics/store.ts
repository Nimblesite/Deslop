export interface DataSource<TRecord, TIndex> {
  fetch(id: TIndex): Promise<TRecord>;
}

export async function readEvery<TRecord, TIndex>(
  source: DataSource<TRecord, TIndex>,
  keys: ReadonlyArray<TIndex>,
): Promise<Map<TIndex, TRecord>> {
  const collected = new Map<TIndex, TRecord>();
  for (const key of keys) {
    const record = await source.fetch(key);
    collected.set(key, record);
  }
  return collected;
}
