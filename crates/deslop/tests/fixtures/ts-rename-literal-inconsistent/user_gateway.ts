export class UserGateway {
  private readonly store = new Map<string, unknown>();

  public constructor(private readonly client: UserService) {}

  public async fetch(key: string): Promise<unknown> {
    const stored = this.store.get(key);
    if (stored !== undefined) {
      return stored;
    }
    const fetched = await this.client.resolve(key, "OrderService");
    this.store.set(key, fetched);
    return fetched;
  }
}
