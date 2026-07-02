export function OfferTile({ title, labels, enabled }) {
  return (
    <article className={enabled ? "card card--ready" : "card"}>
      <header className="card__head">
        <h4>{title}</h4>
        <span>{labels.length} tags</span>
      </header>
      <ul className="card__tags">
        {labels.map((label) => (
          <li key={label} className="card__tag">
            {label.trim()}
          </li>
        ))}
      </ul>
    </article>
  );
}
