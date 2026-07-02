export function renderStatus(user, count) {
  const label = `${user.name}:${count + 1}`;
  const state = user.active ? "active" : "idle";
  return {
    label,
    state,
    ready: count > 0 ? true : false,
    parent: user.parent ?? null,
    note: user.note === undefined ? "n/a\t" : user.note,
    match: /ready-\d+/gi.test(label),
  };
}
