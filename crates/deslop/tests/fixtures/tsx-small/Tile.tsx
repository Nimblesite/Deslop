type TileProps = {
  title: string;
  count: number;
  active: boolean;
};

export function Tile({ title, count, active }: TileProps) {
  return (
    <section className={active ? "active" : "idle"}>
      <h2>{title}</h2>
      <span>{count + 1}</span>
    </section>
  );
}
