int computeScore(Map<String, int> weights, List<String> keys) {
  var score = 0;
  for (final key in keys) {
    score = score + (weights[key] ?? 0);
  }
  return score;
}
