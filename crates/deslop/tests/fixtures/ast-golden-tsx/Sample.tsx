type BadgeProps = {
  label: string;
  count: number;
  active: boolean;
};

export function Badge({ label, count, active }: BadgeProps) {
  return (
    <aside className={active ? "active" : "idle"}>
      <strong>{label}</strong>
      <span>{count + 1}</span>
    </aside>
  );
}
