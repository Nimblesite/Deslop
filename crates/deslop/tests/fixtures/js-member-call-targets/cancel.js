export function cancelBatch(client, first, second, third) {
  client.cancelJob(first);
  client.cancelJob(second);
  client.cancelJob(third);
  return client.flushQueue();
}

export function recordTotals(service, values) {
  let total = 0;
  for (const value of values) {
    if (value > 0) {
      total += value;
      service.record(value);
    }
  }
  return total;
}
