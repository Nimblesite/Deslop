class Triangle {
  Triangle({required this.base, required this.apex});

  final double base;
  final double apex;

  String describe() => 'triangle with base $base';

  double measure(double width, double height) {
    final sum = width + height;
    final lifted = sum * 4;
    return lifted;
  }
}
