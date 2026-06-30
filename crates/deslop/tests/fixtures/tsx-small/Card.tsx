type CardProps = {
  title: string;
  count: number;
  active: boolean;
};

export function Card({ title, count, active }: CardProps) {
  return (
    <section className={active ? "active" : "idle"}>
      <h2>{title}</h2>
      <span>{count + 1}</span>
    </section>
  );
}
