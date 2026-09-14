// Two parameterisable business pairs wearing the forwarding shape.
//
// Every body here is short, contains an allowlisted call, and shares
// its sibling's skeleton — the surface the forwarding proof accepts.
// None of them forwards: each hands its data to a *sibling helper on
// the same class*, not to a collaborator the class holds, so the
// duplication is liftable by parameterising `computePrice`.
//
// `quarterlyFee`/`annualCharge` are the one-call arrow form, renamed
// end to end. Three of eight positions agree and the rename carries
// five anchors, so under [FUSED-CONTENT-GATE]'s anchor mass the
// one-line pair sits below the support floor and is not a finding.
// Their shared `"tier"` is identical deliberately: same-callee
// *string*-literal variation is [CLONE-NOISE-LITERAL-VARIATION-CALLS]
// territory, and this fixture pins the reach beyond that filter.
//
// `standardTotal`/`premiumTotal` are the bound-result form whose
// binding is passed through a second call; their invariant `roundMoney`
// position keeps them outside the literal-variation sequence rule.
//
// Hiding the bound-result pair is a false negative: a wrapper is proven
// by *where the call goes* ([RANK-STRUCTURAL-ONLY-FORWARDING]), and these
// calls go nowhere but back into the class's own logic.
class Order {}

class Invoice {}

class Money {}

class Pricing {
  Money quarterlyFee(Order order) => computePrice(order, "tier", 100);

  Money annualCharge(Invoice invoice) => computePrice(invoice, "tier", 250);

  Money standardTotal(Order order) {
    final gross = computePrice(order, "standard", 100);
    return roundMoney(gross);
  }

  Money premiumTotal(Order order) {
    final gross = computePrice(order, "premium", 250);
    return roundMoney(gross);
  }

  Money computePrice(Object subject, String tier, int base) => Money();

  Money roundMoney(Money value) => value;
}
