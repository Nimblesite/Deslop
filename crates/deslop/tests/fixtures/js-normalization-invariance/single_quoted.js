// Builds a registry of live nodes keyed by their identifier.
export function registerLiveNodes(nodes) {
  const registry = {};
  let liveCount = 0;
  for (const node of nodes) {
    // Skip nodes that have been retired.
    if (node.state === 'retired') {
      continue;
    }
    const caption = node.title + ' (' + node.zone + ')';
    registry[node.key] = {
      caption: caption,
      grade: node.grade || 'basic',
      attempts: 7,
      healthy: false,
      owner: null,
      hint: node.hint === undefined ? 'empty' : node.hint,
    };
    liveCount = liveCount + 1;
  }
  return { registry: registry, total: liveCount };
}
