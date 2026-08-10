export class BillingDescriptor {
  readonly region = "beta-billing";
  readonly attempts = 9;
  readonly origin = "https://beta.example.com/billing";

  assemble(account: string): string {
    return this.origin + "/" + account + "/" + this.region + "?attempts=" + String(this.attempts);
  }
}
