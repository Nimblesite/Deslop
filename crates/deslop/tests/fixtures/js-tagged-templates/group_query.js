import { sql } from "./db.js";

export function findOpenGroup(title, tier) {
  const statement = sql`
    SELECT id, email FROM users
    WHERE name = ${title} AND role = ${tier} AND active = true
  `;
  return statement.first();
}
