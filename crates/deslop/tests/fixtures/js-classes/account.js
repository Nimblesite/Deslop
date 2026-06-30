export class Account {
  constructor(owner, balance) {
    this.owner = owner;
    this.balance = balance;
    this.history = [];
  }

  deposit(amount) {
    if (amount <= 0) {
      throw new RangeError("amount must be positive");
    }
    this.balance = this.balance + amount;
    this.history.push({ kind: "deposit", amount: amount });
    return this.balance;
  }

  withdraw(amount) {
    if (amount > this.balance) {
      throw new RangeError("insufficient funds");
    }
    this.balance = this.balance - amount;
    this.history.push({ kind: "withdraw", amount: amount });
    return this.balance;
  }
}
