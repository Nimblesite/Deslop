import { sql } from "./db.js";

export function findOpenGroup(name, role) {
  const statement = sql`
    SELECT id, email FROM users
    WHERE name = ${name} AND role = ${role} AND active = true
  `;
  return statement.first();
}
