List<double> triangleVertices(double base) {
  return <double>[0.0, base, base / 2];
}

// Shares the name and signature with the sibling `measure` functions
// but has a genuinely different body (the polymorphic pattern).
double measure(double width, double height) {
  final sum = width + height;
  final lifted = sum * 4;
  return lifted;
}
