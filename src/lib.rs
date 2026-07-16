//! # golden-bridge
//!
//! The bridge between the legacy `schema-language` corpus and the next-generation
//! language family. It does one thing: ingest a real legacy schema source into the
//! new stringless [`CoreSchema`](core_schema::CoreSchema) +
//! [`NameTable`](name_table::NameTable) model, so a real legacy fixture can be
//! migrated through the whole new vertical
//! (`CoreSchema → core-nomos macros → CoreLogos → TextualRust → Rust`) and
//! byte-compared against the Rust the legacy generator already emits.
//!
//! This is the one place the legacy parser and the new vertical legitimately meet;
//! keeping it a separate component keeps the legacy dependency out of the clean
//! new-Core crates. See the crate ARCHITECTURE for the leg-one subset boundary and
//! the acceptance witness (`tests/spirit_bridge.rs`).

pub mod error;
pub mod ingest;

pub use error::BridgeError;
pub use ingest::{Exclusion, LegacySchemaIngest, Migration};
