export function ProductCard({ name, tags, available }) {
  return (
    <article className={available ? "card card--ready" : "card"}>
      <header className="card__head">
        <h4>{name}</h4>
        <span>{tags.length} tags</span>
      </header>
      <ul className="card__tags">
        {tags.map((tag) => (
          <li key={tag} className="card__tag">
            {tag.trim()}
          </li>
        ))}
      </ul>
    </article>
  );
}
