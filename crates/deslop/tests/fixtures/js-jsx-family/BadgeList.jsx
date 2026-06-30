function buildBadgeModel(notifications) {
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

export function BadgeList({ notifications }) {
  const model = buildBadgeModel(notifications);
  return (
    <ul className="badges">
      {model.map((badge) => (
        <li key={badge.channel}>
          {badge.channel}: {badge.count}
        </li>
      ))}
    </ul>
  );
}
