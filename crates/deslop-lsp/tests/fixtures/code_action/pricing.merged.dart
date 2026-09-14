void mergedFromCluster_22921e(List<String> book, String arg0, int arg1) {
  final label = arg0;
  final ceiling = arg1;
  book.add(label);
  book.add(label.toUpperCase());
  book.add(label.toLowerCase());
  book.add(label.trim());
  book.add(ceiling.toString());
  book.sort();
}

void applyStandard(List<String> book) {
  mergedFromCluster_22921e(book, "standard", 100);
}

void applyPremium(List<String> book) {
  mergedFromCluster_22921e(book, "premium", 250);
}
