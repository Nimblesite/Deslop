export class AuditDescriptor {
  readonly ledger = "delta-audit";
  readonly depth = 128;
  readonly archive = "https://delta.example.com/audit";

  render(actor: string): string {
    return this.archive + "/" + actor + "/" + this.ledger + "?depth=" + String(this.depth);
  }
}
