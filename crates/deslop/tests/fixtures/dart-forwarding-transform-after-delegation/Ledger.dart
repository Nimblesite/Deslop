// A collaborator call is not a licence for the rest of the body.
//
// Both methods hand a request to the injected `client` — the delegation
// the forwarding proof looks for — and then run the response through a
// *sibling helper on the same class*, parameterised by the literals that
// differ between them. Everything the pair could lift lives in that
// second call: `applyMarkup(gross, "standard", 100)` beside
// `applyMarkup(gross, "premium", 250)` is one `applyMarkup(gross, tier,
// base)` away from being a single method.
//
// The delegating call is byte-identical in both bodies, so it carries no
// duplication at all. Proving that *a* call forwards and then accepting
// every other call in the body reads the one statement that is not the
// duplication and excuses the one that is
// ([RANK-STRUCTURAL-ONLY-FORWARDING]).
//
// Contrast `dart-issue-197-settings-getters`, where the extra call
// consumes the delegated response and nothing else: `_getTask(http
// .deleteMethod(route))` and `IndexSettings.fromMap(response.data!)`
// compute nothing the class owns. The difference is not how many calls a
// body makes — it is whether a call reaches back into the class's own
// members.
class Order {}

class Money {}

class PriceApi {
  Money fetch(Order order) => Money();
}

class Ledger {
  final PriceApi client;

  Ledger(this.client);

  Money standardTotal(Order order) {
    final gross = client.fetch(order);
    return applyMarkup(gross, "standard", 100);
  }

  Money premiumTotal(Order order) {
    final gross = client.fetch(order);
    return applyMarkup(gross, "premium", 250);
  }

  Money applyMarkup(Money value, String tier, int base) => value;
}
