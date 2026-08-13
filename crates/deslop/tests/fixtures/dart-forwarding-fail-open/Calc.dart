// Two liftable Dart methods, each exactly ONE statement, each containing
// a call — the same surface shape as the meilisearch REST wrappers.
//
// They are not wrappers. Each computes: a multiplication and an addition
// sit between the input and the call. `[RANK-STRUCTURAL-ONLY-FORWARDING]`
// admits only a closed declarative allowlist, and arithmetic is not on
// it, so neither body can prove forwarding and the pair stays visible.
//
// A statement count would hide both. That is the mistake this fixture
// exists to catch.
class Calc {
  int scaledDomestic(List<int> amounts, int rate) {
    return record(amounts.length * rate + 7);
  }

  int scaledExport(List<int> rows, int factor) {
    return record(rows.length * factor + 7);
  }

  int record(int v) => v;
}
