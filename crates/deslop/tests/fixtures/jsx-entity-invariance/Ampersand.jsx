export function Ampersand({ left, right }) {
  return (
    <span className="join">
      <em>{left}</em>
      <b>&amp;</b>
      <em>{right}</em>
    </span>
  );
}
