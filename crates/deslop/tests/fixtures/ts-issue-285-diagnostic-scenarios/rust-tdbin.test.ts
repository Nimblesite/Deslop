// Seven independent negative scenarios. Each documents a different error
// contract of the binary codec; they share only the ordinary test idiom
// — build a schema, run the codec, hand the result to the existing
// `expectErrorMessages` helper. Parameterising them would hide the
// contracts they exist to document.

test("rejects an unsupported field type", () => {
  const schema = buildSchema({ kind: "record", fields: [{ name: "at", type: "Duration" }] });
  const result = encodeTdbin(schema);
  expectErrorMessages(result, ["field type is not supported by the binary codec"]);
});

test("rejects Option over an unsupported type", () => {
  const schema = buildSchema({ kind: "record", fields: [{ name: "at", type: "Option<Duration>" }] });
  const result = encodeTdbin(schema);
  expectErrorMessages(result, ["field type is not supported by the binary codec"]);
});

test("rejects a typed map key", () => {
  const schema = buildSchema({ kind: "record", fields: [{ name: "index", type: "Map<Point, i32>" }] });
  const result = encodeTdbin(schema);
  expectErrorMessages(result, ["typed map keys must be scalars"]);
});

test("rejects an Any-typed field", () => {
  const schema = buildSchema({ kind: "record", fields: [{ name: "payload", type: "Any" }] });
  const result = encodeTdbin(schema);
  expectErrorMessages(result, ["Any is not representable in the binary codec"]);
});

test("rejects an empty record list", () => {
  const schema = buildSchema({ kind: "records", records: [] });
  const result = encodeTdbin(schema);
  expectErrorMessages(result, ["record list must declare at least one record"]);
});

test("rejects an unmonomorphized generic", () => {
  const schema = buildSchema({ kind: "generic", parameters: ["T"], monomorphized: false });
  const result = encodeTdbin(schema);
  expectErrorMessages(result, ["generic record reached the codec unmonomorphized"]);
});

test("rejects an undeclared union variant", () => {
  const schema = buildSchema({ kind: "union", variants: ["Alpha"], selected: "Omega" });
  const result = encodeTdbin(schema);
  expectErrorMessages(result, ["union variant is not declared on the union"]);
});
