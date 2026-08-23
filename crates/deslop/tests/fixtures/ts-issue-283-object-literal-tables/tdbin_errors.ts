// TDBIN error metadata: diagnostic code on the left, the human-facing
// message template on the right. Owned by the codec, not by rendering
// and not by any converter.

export const tdbinErrors = {
  unsupportedField: "field type is not supported by the binary codec",
  unmonomorphized: "generic record reached the codec unmonomorphized",
  emptyRecordList: "record list must declare at least one record",
  typedMapKey: "typed map keys must be scalars",
  anyDiagnostic: "Any is not representable in the binary codec",
  invalidVariant: "union variant is not declared on the union",
};
