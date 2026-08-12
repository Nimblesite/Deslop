import { ReactElement } from "react";

type UserPanelProps = {
  caption: string;
  members: string[];
  featured: boolean;
};

export function UserPanel({ caption, members, featured }: UserPanelProps): ReactElement {
  return (
    <section className={featured ? "panel panel--lit" : "panel"}>
      <header className="panel__head">
        <h3>{caption}</h3>
        <small>{members.length} items</small>
      </header>
      <ul className="panel__list">
        {members.map((member) => (
          <li key={member} className="panel__item">
            {member.toUpperCase()}
          </li>
        ))}
      </ul>
    </section>
  );
}
