Map<String, int> tally(List<String> words) {
  final counts = <String, int>{};
  for (final word in words) {
    counts[word] = (counts[word] ?? 0) + 1;
  }
  return counts;
}
