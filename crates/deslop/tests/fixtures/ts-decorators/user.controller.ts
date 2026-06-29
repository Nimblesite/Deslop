function Controller(prefix: string): ClassDecorator {
  return () => undefined;
}
function Inject(token: string): ParameterDecorator {
  return () => undefined;
}

@Controller("/users")
export abstract class UserController {
  protected readonly cache: Map<string, unknown> = new Map();

  public constructor(@Inject("UserService") private readonly service: UserService) {}

  public async findOne(id: string): Promise<unknown> {
    const cached = this.cache.get(id);
    if (cached !== undefined) {
      return cached;
    }
    const loaded = await this.service.resolve(id);
    this.cache.set(id, loaded);
    return loaded;
  }

  protected abstract authorize(id: string): boolean;
}

interface UserService {
  resolve(id: string): Promise<unknown>;
}
