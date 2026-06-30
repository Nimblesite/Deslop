function Controller(prefix: string): ClassDecorator {
  return () => undefined;
}
function Inject(token: string): ParameterDecorator {
  return () => undefined;
}

@Controller("/orders")
export abstract class OrderController {
  protected readonly store: Map<string, unknown> = new Map();

  public constructor(@Inject("OrderService") private readonly gateway: OrderService) {}

  public async findOne(key: string): Promise<unknown> {
    const hit = this.store.get(key);
    if (hit !== undefined) {
      return hit;
    }
    const fetched = await this.gateway.resolve(key);
    this.store.set(key, fetched);
    return fetched;
  }

  protected abstract authorize(key: string): boolean;
}

interface OrderService {
  resolve(id: string): Promise<unknown>;
}
