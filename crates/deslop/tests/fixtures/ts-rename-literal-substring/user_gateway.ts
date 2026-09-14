export class UserGateway {
  private readonly store = new Map<string, unknown>();

  public constructor(private readonly client: UserService) {}

  public async fetch(key: string): Promise<unknown> {
    const stored = this.store.get(key);
    if (stored !== undefined) {
      return stored;
    }
    if (key.length === 0) {
      throw new Error("invalkey request");
    }
    const fetched = await this.client.resolve(key, "UserService");
    this.store.set(key, fetched);
    return fetched;
  }
}
