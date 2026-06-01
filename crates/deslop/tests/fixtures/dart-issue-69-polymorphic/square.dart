int squareArea(int side, int border) {
  return (side + border) * (side + border);
}

// Shares the name and signature with the sibling `measure` functions
// but has a genuinely different body (the polymorphic pattern).
double measure(double width, double height) {
  final amount = width + height;
  final boosted = amount * 3;
  return boosted;
}
