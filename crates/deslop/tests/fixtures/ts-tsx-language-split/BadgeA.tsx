type BadgeProps = {
  label: string;
  tone: "info" | "warn" | "error";
};

export function Badge({ label, tone }: BadgeProps) {
  return (
    <span className={`badge badge-${tone}`} role="status">
      <i className={`icon icon-${tone}`} aria-hidden="true" />
      {label}
    </span>
  );
}
