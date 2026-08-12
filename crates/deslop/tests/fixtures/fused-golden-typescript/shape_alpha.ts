export class InventoryDescriptor {
  readonly channel = "alpha-inventory";
  readonly retries = 3;
  readonly endpoint = "https://alpha.example.com/inventory";

  compose(tenant: string): string {
    return this.endpoint + "/" + tenant + "/" + this.channel + "?retries=" + String(this.retries);
  }
}
