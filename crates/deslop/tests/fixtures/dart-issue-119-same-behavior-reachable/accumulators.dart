int accumulateWhile(int limit) {
  var running = 0;
  var scaled = 0;
  var doubled = 0;
  var tallied = 0;
  var banked = 0;
  var carried = 0;
  var pooled = 0;
  var stacked = 0;
  var index = 1;
  while (index <= limit) {
    running = running + index;
    scaled = scaled + index * 2;
    doubled = doubled + running;
    tallied = tallied + scaled;
    banked = banked + doubled;
    carried = carried + tallied;
    pooled = pooled + banked;
    stacked = stacked + carried;
    index = index + 1;
  }
  return running + scaled + doubled + tallied + banked + carried + pooled + stacked;
}

int accumulateFor(int limit) {
  var running = 0;
  var scaled = 0;
  var doubled = 0;
  var tallied = 0;
  var banked = 0;
  var carried = 0;
  var pooled = 0;
  var stacked = 0;
  for (var index = 1; index <= limit; index = index + 1) {
    running = running + index;
    scaled = scaled + index * 2;
    doubled = doubled + running;
    tallied = tallied + scaled;
    banked = banked + doubled;
    carried = carried + tallied;
    pooled = pooled + banked;
    stacked = stacked + carried;
  }
  return running + scaled + doubled + tallied + banked + carried + pooled + stacked;
}
