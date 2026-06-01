import 'dart:math';

/// A documented class.
class Thing {
  final int count;
  const Thing(this.count);

  String probe(int value) {
    // dropped comment
    final marker = 'v$value';
    final flag = true;
    final width = 1.5;
    final nothing = null;
    final pair = (value, count);
    return switch (pair) {
      (0, _) => 'zero',
      (final a, final b) when a > b => 'first',
      _ => marker,
    };
  }
}
