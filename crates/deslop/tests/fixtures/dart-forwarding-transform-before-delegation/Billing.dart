// The same hole reached from the other side: compute first, then hand
// the computed value to the collaborator.
//
// `client.submit(...)` delegates, so a proof that stops at "some call in
// this body reaches collaborator state" is satisfied. What it submits is
// `normalise(order, tier, base)` — a sibling helper on this class,
// applied to the member's *own parameter* and parameterised by the two
// literals that differ. That call is the duplication, and it runs before
// anything leaves the class. Parameterising `normalise` collapses the
// pair into one method.
//
// This shape is strictly worse than the post-delegation one: the class
// computes on its own inputs. No REST wrapper does that — the
// meilisearch family passes route literals and request bodies straight
// through, and every non-delegating call it makes consumes only what the
// client already returned ([RANK-STRUCTURAL-ONLY-FORWARDING]).
//
// The two members keep identical parameter and binding names and differ
// only in the method name and the two literals, which is what lands the
// pair in `structural_only` — the bucket where the declaration-family
// filter actually runs. A consistent end-to-end rename would route it to
// `nearly_identical` instead and the test would pass without ever
// reaching the proof under repair.
class Order {}

class Money {}

class BillingApi {
  Money submit(Money amount) => amount;
}

class Billing {
  final BillingApi client;

  Billing(this.client);

  Money quarterlyFee(Order order) {
    final submitted = client.submit(normalise(order, "standard", 100));
    return submitted;
  }

  Money annualCharge(Order order) {
    final submitted = client.submit(normalise(order, "premium", 250));
    return submitted;
  }

  Money normalise(Object subject, String tier, int base) => Money();
}
