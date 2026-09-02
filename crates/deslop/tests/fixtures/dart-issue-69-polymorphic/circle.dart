void logCircle(String message) {
  print(message);
}

// Shares the name and signature with the sibling `measure` overrides
// but implements it with a genuinely different body shape (the
// polymorphic pattern). The `@override` marker carries the contract.
class Circle {
  @override
  double measure(double width, double height) {
    final total = width + height;
    return total * 2;
  }
}
