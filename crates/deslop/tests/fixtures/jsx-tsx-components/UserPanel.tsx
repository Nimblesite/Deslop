import { ReactElement } from "react";

type UserPanelProps = {
  heading: string;
  badges: string[];
  highlighted: boolean;
};

export function UserPanel({ heading, badges, highlighted }: UserPanelProps): ReactElement {
  return (
    <section className={highlighted ? "panel panel--lit" : "panel"}>
      <header className="panel__head">
        <h3>{heading}</h3>
        <small>{badges.length} items</small>
      </header>
      <ul className="panel__list">
        {badges.map((badge) => (
          <li key={badge} className="panel__item">
            {badge.toUpperCase()}
          </li>
        ))}
      </ul>
    </section>
  );
}
