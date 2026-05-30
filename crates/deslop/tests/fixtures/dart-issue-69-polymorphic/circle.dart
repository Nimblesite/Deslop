class Circle {
  Circle(this.radius);

  final double radius;

  double measure(double width, double height) {
    final total = width + height;
    final scaled = total * 2;
    return scaled;
  }
}
