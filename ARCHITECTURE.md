# golden-bridge

The bridge from the legacy `schema-language` corpus to the next-generation
language family. One capability: ingest a real legacy schema source into the new
stringless `CoreSchema` + `NameTable` model, so a real legacy fixture can be
migrated through the whole new vertical and byte-compared against the Rust the
legacy generator already emits.

## Why a separate component

This is the single place where the legacy parser and the new vertical legitimately
meet. Housing it here — rather than in `core-schema` or `core-nomos` — keeps the
legacy `schema-language` dependency out of the clean new-Core crates: `core-schema`
stays a stringless target with no knowledge of the corpus it is eventually fed
from. The bridge depends inward on both worlds and exports the migration; nothing
depends back on it.

## The vertical it drives

```
legacy .schema text
  → schema-language parser (SchemaSource)
  → LegacySchemaIngest              (this crate: legacy model → new Core)
  → CoreSchema + NameTable
  → core-nomos MacroPackage::apply  → CoreLogos
  → TextualRust                     → Rust text
  → byte-compare against the legacy generated Rust
```

Parsing is the legacy parser's job; this crate never hand-rolls a parser. It
translates the already-parsed legacy model object-to-object into the new Core and
carries no strings into it — every legacy name is re-interned into the new
`NameTable` and the declarations hold only identifiers.

## Leg-one subset boundary

Leg one migrates the ordinary type-declaration core:

- a newtype over a scalar (`String`, `Integer`, `Boolean`, `Bytes`) or a plain
  reference to a declared type;
- a struct whose fields are scalar or plain references, elided (name derived from
  the type) or explicit.

Everything beyond is a loud, named `BridgeError`, collected as an `Exclusion`
rather than guessed at:

- generic applications (`Vector.T`, `Map.(K V)`, `Optional.T`, `Bytes.N`);
- enum declarations;
- pipe-text declarations;
- inline field declarations;
- the legacy `Path` scalar, which the new `CoreReference` has no case for.

An elided field defers its stored name to the reference's single-homed
`derived_field_name`, so the derive-versus-preserve decision stays owned by
`core-schema`'s one elision predicate rather than being re-implemented here.

## Visibility

The legacy schema surface carries no visibility token, so every migrated
declaration is public API intent. `core-nomos` then lowers that coarse
declaration visibility faithfully onto the produced item — a settled ruling
(schema visibility is an authoritative API promise), so the fidelity is real
rather than a projection coincidence.

## Acceptance witness

`tests/spirit_bridge.rs` migrates the real `spirit-min.schema` and asserts that
its six in-subset declarations (`Topic`, `Description`, `Summary`,
`RecordIdentifier`, `Entry`, `Query`) reproduce the legacy golden bytes exactly,
and that the four out-of-subset declarations are named exclusions with the correct
typed reason. The fixtures are byte-exact vendored copies of the legacy corpus
(see `tests/fixtures/PROVENANCE.md`). The witness is the full `nix flake check`.

## Status

Prototype, leg one. The subset boundary, the choice of fixture, and the housing
are agent leans taken to reach a working prototype; all are cheap to revise as the
next legs widen the subset (enums, generic applications, aliases, imports).
