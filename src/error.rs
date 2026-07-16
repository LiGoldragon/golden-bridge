//! The bridge's typed error boundary. Everything the leg-one ingestion cannot
//! faithfully carry into the new stringless Core is a loud, named variant — never
//! a silent skip or a guess. The migration harness partitions declarations into
//! the migrated set and the excluded set, and each exclusion carries one of these
//! reasons verbatim, so an out-of-subset construct is always visible and named.

use thiserror::Error;

/// A failure to ingest a legacy schema declaration into the new Core, or to lower
/// a migrated declaration through the vertical. The unsupported-construct variants
/// are the leg-one subset boundary: newtypes and structs over scalar and plain
/// references are in; everything else is a named exclusion.
#[derive(Debug, Error)]
pub enum BridgeError {
    /// The legacy `schema-language` parser rejected the source text.
    #[error("legacy schema parse failed: {0:?}")]
    LegacyParse(schema_language::SchemaError),

    /// A scalar leaf the new `CoreReference` has no case for. The new Core carries
    /// String, Integer, Boolean and Bytes; the legacy `Path` scalar has no home
    /// here and is named rather than coerced.
    #[error("unsupported scalar leaf `{0}`: the new Core has no such reference")]
    UnsupportedScalar(String),

    /// A generic application (`Vector.T`, `Map.(K V)`, `Optional.T`, `Bytes.N`).
    /// Leg one migrates only scalar and plain references; applications are named,
    /// not lowered.
    #[error(
        "unsupported generic application in `{0}`: leg one migrates scalar and plain references only"
    )]
    UnsupportedApplication(String),

    /// An enum declaration. Leg one migrates newtypes and structs only.
    #[error("unsupported enum declaration `{0}`: leg one migrates newtypes and structs only")]
    UnsupportedEnum(String),

    /// A pipe-text (documentation) declaration value.
    #[error("unsupported text declaration `{0}`")]
    UnsupportedText(String),

    /// A struct field whose value is an inline nested declaration rather than a
    /// reference.
    #[error("unsupported inline field declaration in struct `{0}`")]
    UnsupportedInlineField(String),

    /// A NameTable resolution failure while deriving an elided field name.
    #[error("name table error: {0}")]
    NameTable(#[from] name_table::NameTableError),
}

impl From<schema_language::SchemaError> for BridgeError {
    fn from(error: schema_language::SchemaError) -> Self {
        Self::LegacyParse(error)
    }
}
