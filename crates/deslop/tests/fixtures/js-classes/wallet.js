export class Wallet {
  constructor(holder, funds) {
    this.holder = holder;
    this.funds = funds;
    this.ledger = [];
  }

  credit(value) {
    if (value <= 0) {
      throw new RangeError("value must be positive");
    }
    this.funds = this.funds + value;
    this.ledger.push({ type: "credit", value: value });
    return this.funds;
  }

  debit(value) {
    if (value > this.funds) {
      throw new RangeError("not enough money");
    }
    this.funds = this.funds - value;
    this.ledger.push({ type: "debit", value: value });
    return this.funds;
  }
}
