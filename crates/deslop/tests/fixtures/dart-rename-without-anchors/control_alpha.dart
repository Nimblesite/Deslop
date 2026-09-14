int mergeTotals(List<int> counts, List<int> offsets) {
  var total = 0;
  var carry = 1;
  for (final count in counts) {
    final scaled = count * carry;
    final shifted = scaled + offsets.length;
    total = total + shifted;
    carry = carry + 1;
  }
  for (final offset in offsets) {
    final damped = offset - carry;
    final folded = damped * 2;
    total = total - folded;
    carry = carry * 1;
  }
  var checksum = 0;
  for (final count in counts) {
    final mixed = count + carry;
    final spun = mixed * total;
    checksum = checksum + spun;
  }
  return total + carry + checksum;
}
