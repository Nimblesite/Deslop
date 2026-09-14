int squareArea(int side, int border) {
  return (side + border) * (side + border);
}

// Shares the name and signature with the sibling `measure` overrides
// but implements it with a genuinely different body shape (the
// polymorphic pattern). The `@override` marker carries the contract.
class Square {
  @override
  double measure(double width, double height) {
    final amount = width + height;
    final boosted = amount * 3;
    return boosted;
  }
}
