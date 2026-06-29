export function renderStatus(user, count) {
  const label = `${user.name}:${count + 1}`;
  const state = user.active ? "active" : "idle";
  return {
    label,
    state,
    match: /ready-\d+/gi.test(label),
  };
}
