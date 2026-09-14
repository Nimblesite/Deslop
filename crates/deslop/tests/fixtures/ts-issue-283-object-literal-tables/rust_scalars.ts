// Directional scalar map for the Rust converter: TypeDiagram scalar name
// on the left, the Rust type that carries it on the right.

export const rustScalars = {
  string: "String",
  boolean: "bool",
  int32: "i32",
  int64: "i64",
  float64: "f64",
  timestamp: "chrono::DateTime<chrono::Utc>",
};

export const rustDefaults = {
  string: "String::new()",
  boolean: "false",
  int32: "0",
  int64: "0",
  float64: "0.0",
  timestamp: "chrono::Utc::now()",
};
