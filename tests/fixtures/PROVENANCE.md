# Fixture provenance

These two files are byte-exact vendored copies from the legacy corpus, kept here
so the bridge's acceptance witness is self-contained and portable.

- `spirit-min.schema` — the legacy schema source. Copied verbatim from
  `schema-rust/tests/fixtures/spirit-min.schema` at
  `schema-rust @ 87de872dbc4ee124a6a1133ff520b594063304f5`.
- `spirit_generated.rs` — the Rust the legacy generator emits for that source.
  Copied verbatim from `schema-rust/tests/fixtures/spirit_generated.rs` at the
  same commit. In the legacy repo this file is the byte-exact snapshot asserted by
  `assert_generated_fixture("spirit_generated.rs", …)` (schema-rust
  `tests/emission.rs`).

The bridge parses the source through `schema-language @ 59d59aca5767` and migrates
its in-subset declarations through the next-generation vertical, then byte-compares
each migrated item against its block in the golden. The comparison is per item, as
textual-rust established: the six in-subset declarations must match their golden
bytes exactly; the four out-of-subset declarations (generic applications, enums)
are named exclusions.
