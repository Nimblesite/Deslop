export function launchBatch(client, first, second, third) {
  client.startJob(first);
  client.startJob(second);
  client.startJob(third);
  return client.flushQueue();
}

export function recordTotals(client, values) {
  let total = 0;
  for (const value of values) {
    if (value > 0) {
      total += value;
      client.record(value);
    }
  }
  return total;
}
