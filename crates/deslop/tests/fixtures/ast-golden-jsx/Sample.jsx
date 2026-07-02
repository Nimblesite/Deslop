export function Notice({ title, items, urgent }) {
  return (
    <div className={urgent ? "notice urgent" : "notice"}>
      <h2>Heads &amp; Shoulders</h2>
      <p>{title}</p>
      <small>&copy; 2026 Acme</small>
      <ul>
        {items.map((item) => (
          <li key={item}>{item}</li>
        ))}
      </ul>
    </div>
  );
}
