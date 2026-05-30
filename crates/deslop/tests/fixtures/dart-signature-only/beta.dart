int computeScore(Map<String, int> table, List<String> names) {
  final buffer = StringBuffer();
  names.sort();
  buffer.writeAll(names, ',');
  return buffer.length;
}
