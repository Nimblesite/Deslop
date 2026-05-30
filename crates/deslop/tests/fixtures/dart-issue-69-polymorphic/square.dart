class Square {
  Square(this.side, this.color);

  final double side;
  final String color;

  bool get isLarge => side > 10;

  double measure(double width, double height) {
    final amount = width + height;
    final boosted = amount * 3;
    return boosted;
  }
}
