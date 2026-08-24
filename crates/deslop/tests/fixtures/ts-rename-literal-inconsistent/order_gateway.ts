export class OrderGateway {
  private readonly cache = new Map<string, unknown>();

  public constructor(private readonly service: OrderService) {}

  public async fetch(id: string): Promise<unknown> {
    const cached = this.cache.get(id);
    if (cached !== undefined) {
      return cached;
    }
    const loaded = await this.service.resolve(id, "OrderService");
    this.cache.set(id, loaded);
    return loaded;
  }
}
