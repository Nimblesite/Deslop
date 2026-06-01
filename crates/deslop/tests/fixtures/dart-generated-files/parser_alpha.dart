int tallyVotes(List<String> ballots) {
  var yes = 0;
  var no = 0;
  for (final ballot in ballots) {
    if (ballot == 'yes') {
      yes = yes + 1;
    } else {
      no = no + 1;
    }
  }
  return yes - no;
}
