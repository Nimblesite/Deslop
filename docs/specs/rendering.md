# Report rendering

## [CLI-TEXT] Text renders the engine report

The text report renders the same cluster membership, mass, and repository figures as [OUTPUT-SCHEMA-JSON]. Its output is ASCII and line-oriented. It does not derive a second set of scores or percentages. `crates/deslop-core/src/render/text.rs` implements this projection; `crates/deslop/tests/dart_forwarding_fail_open.rs` asserts the rendered text against the fixture's report facts.

## [LOCATION-LINE-COLUMN] Human occurrence locations

Human report rows display file paths and line-based locations, never raw byte ranges. The text renderer prints the engine's `path:start_line:end_line` span. Editor navigation may display the starting `path:line:column`; byte offsets remain available in the machine report for exact endpoint identity. The renderer consumes the engine's location fields without treating a byte offset as a line or column.

`crates/deslop-core/src/render/text.rs` renders text spans; `crates/deslop/tests/location_rendering.rs` drives the CLI and checks the text and HTML location surfaces. Path separators follow [OUTPUT-SCHEMA-PATH-SEPARATOR].
