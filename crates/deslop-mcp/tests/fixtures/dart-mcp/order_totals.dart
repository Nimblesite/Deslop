double computeOrderTotal(List<OrderLine> lines, double levy, double rebate) {
  var running = 0.0;
  for (final line in lines) {
    final entryTotal = line.price * line.count;
    if (line.discounted) {
      running = running + entryTotal * 0.9;
    } else {
      running = running + entryTotal;
    }
  }
  final withLevy = running * (1 + levy);
  final afterRebate = withLevy - rebate;
  return afterRebate < 0 ? 0 : afterRebate;
}
