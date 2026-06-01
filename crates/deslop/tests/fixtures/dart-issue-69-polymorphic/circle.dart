void logCircle(String message) {
  print(message);
}

// Shares the name and signature with the sibling `measure` functions
// but has a genuinely different body (the polymorphic pattern).
double measure(double width, double height) {
  final total = width + height;
  final scaled = total * 2;
  return scaled;
}
