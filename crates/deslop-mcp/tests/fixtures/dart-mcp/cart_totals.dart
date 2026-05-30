double computeCartTotal(List<LineItem> items, double taxRate, double discount) {
  var subtotal = 0.0;
  for (final item in items) {
    final lineTotal = item.unitPrice * item.quantity;
    if (item.onSale) {
      subtotal = subtotal + lineTotal * 0.9;
    } else {
      subtotal = subtotal + lineTotal;
    }
  }
  final taxed = subtotal * (1 + taxRate);
  final afterDiscount = taxed - discount;
  return afterDiscount < 0 ? 0 : afterDiscount;
}
