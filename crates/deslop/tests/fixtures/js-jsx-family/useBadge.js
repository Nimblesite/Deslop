export function buildBadgeModel(notifications) {
  const grouped = new Map();
  for (const notification of notifications) {
    const bucket = grouped.get(notification.channel) ?? [];
    bucket.push(notification.message);
    grouped.set(notification.channel, bucket);
  }
  const summary = [];
  for (const [channel, messages] of grouped) {
    summary.push({ channel, count: messages.length });
  }
  return summary.sort((left, right) => right.count - left.count);
}
