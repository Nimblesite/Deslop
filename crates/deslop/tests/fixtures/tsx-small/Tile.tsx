type TileProps = {
  label: string;
  total: number;
  enabled: boolean;
};

export function Tile({ label, total, enabled }: TileProps) {
  return (
    <section className={enabled ? "active" : "idle"}>
      <h2>{label}</h2>
      <span>{total + 1}</span>
    </section>
  );
}
