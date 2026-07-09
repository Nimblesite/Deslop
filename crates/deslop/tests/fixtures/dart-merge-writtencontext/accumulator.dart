int total = 0;

void growStandard(List<String> book) {
  final label = "standard";
  total = total + 1;
  book.add(label);
  book.add(label.toUpperCase());
  book.add(label.toLowerCase());
  book.add(label.trim());
  book.add(total.toString());
  book.sort();
}

void growPremium(List<String> book) {
  final label = "premium";
  total = total + 1;
  book.add(label);
  book.add(label.toUpperCase());
  book.add(label.toLowerCase());
  book.add(label.trim());
  book.add(total.toString());
  book.sort();
}
