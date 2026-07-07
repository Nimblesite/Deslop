void applyStandard(List<String> book) {
  final label = "standard";
  final ceiling = 100;
  book.add(label);
  book.add(label.toUpperCase());
  book.add(label.toLowerCase());
  book.add(label.trim());
  book.add(ceiling.toString());
  book.sort();
}

void applyPremium(List<String> book) {
  final label = "premium";
  final ceiling = 250;
  book.add(label);
  book.add(label.toUpperCase());
  book.add(label.toLowerCase());
  book.add(label.trim());
  book.add(ceiling.toString());
  book.sort();
}
