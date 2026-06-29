export type Status = {
  name: string;
  count: number;
  active?: boolean;
};

export function renderStatus(user: Status): string {
  const label = `${user.name}:${user.count + 1}`;
  return user.active ? label : "idle";
}
