// One integration scenario: TypeScript Option scalar layout generation.
// It shares only the produce-then-assert test shape with the Rust
// scenarios in the sibling file; its input, output and assertions are
// its own.

test("generates the Option scalar layout for TypeScript", () => {
  const schema = loadFixture("option-scalars.td");
  const generated = generateTypeScript(schema, { tdbin: true });
  expect(generated).toContain("export type OptionScalars = {");
  expect(generated).toContain("readonly maybeCount?: number;");
  expect(generated).toContain("readonly maybeLabel?: string;");
  expect(generated).toContain("export const OPTION_SCALARS_LAYOUT = [");
  expect(generated).toContain("{ field: \"maybeCount\", width: 4, nullable: true }");
  expect(generated).toContain("{ field: \"maybeLabel\", width: 0, nullable: true }");
});
