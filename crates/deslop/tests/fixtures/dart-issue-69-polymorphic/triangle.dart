List<double> triangleVertices(double base) {
  return <double>[0.0, base, base / 2];
}

// Shares the name and signature with the sibling `measure` overrides
// but implements it with a genuinely different body shape (the
// polymorphic pattern). The `@override` marker carries the contract.
class Triangle {
  @override
  double measure(double width, double height) {
    var total = width + height;
    for (var step = 0; step < 3; step++) {
      total = total + step;
    }
    return total;
  }
}
