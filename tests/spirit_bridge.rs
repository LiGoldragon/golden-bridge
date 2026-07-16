//! Bridge leg one, end to end: a real legacy schema source, migrated through the
//! whole next-generation vertical, byte-exact against the real legacy generated
//! Rust.
//!
//! `spirit-min.schema` and `spirit_generated.rs` are byte-exact vendored copies of
//! the legacy corpus (schema-rust @ 87de872 — see `fixtures/PROVENANCE.md`). The
//! flow: legacy source text → `LegacySchemaIngest` → new CoreSchema + NameTable →
//! `core-nomos` macros → CoreLogos → `TextualRust` → Rust, then a byte comparison
//! against the corresponding item block in the legacy golden.
//!
//! The byte-compare unit is the item, as textual-rust established: the migrated
//! in-subset declarations must reproduce their legacy golden bytes exactly, while
//! the out-of-subset declarations (generic applications, enums) are named
//! exclusions, never silent drops.

use core_logos::CoreItem;
use core_nomos::MacroPackage;
use golden_bridge::{BridgeError, LegacySchemaIngest};
use name_table::NameTable;
use textual_rust::RustSource;

const LEGACY_SOURCE: &str = include_str!("fixtures/spirit-min.schema");
const LEGACY_GOLDEN: &str = include_str!("fixtures/spirit_generated.rs");

/// The declarations the leg-one subset migrates, in schema order.
const MIGRATED: [&str; 6] = [
    "Topic",
    "Description",
    "Summary",
    "RecordIdentifier",
    "Entry",
    "Query",
];

/// Project one lowered item to Rust text.
fn project(item: &CoreItem, names: &NameTable) -> String {
    RustSource::project_item(item, names)
        .expect("project item")
        .as_str()
        .to_owned()
}

/// Split the legacy golden file into item blocks keyed by declared type name. Each
/// block is a run of non-blank lines separated by a blank line; the value is the
/// block plus its trailing newline, matching the per-item projection bytes.
fn golden_item_blocks(golden: &str) -> Vec<(String, String)> {
    let mut blocks = Vec::new();
    for paragraph in golden.split("\n\n") {
        let trimmed = paragraph.trim_matches('\n');
        if trimmed.is_empty() {
            continue;
        }
        if let Some(name) = declared_name(trimmed) {
            blocks.push((name, format!("{trimmed}\n")));
        }
    }
    blocks
}

/// The declared type name in a golden block, if it declares a struct or enum.
fn declared_name(block: &str) -> Option<String> {
    for line in block.lines() {
        for head in ["pub struct ", "pub enum "] {
            if let Some(rest) = line.strip_prefix(head) {
                let name: String = rest
                    .chars()
                    .take_while(|character| character.is_alphanumeric() || *character == '_')
                    .collect();
                if !name.is_empty() {
                    return Some(name);
                }
            }
        }
    }
    None
}

#[test]
fn spirit_min_migrates_byte_exact_against_the_legacy_golden() {
    // Legacy source text -> new CoreSchema + NameTable.
    let migration = LegacySchemaIngest::migrate_text(LEGACY_SOURCE).expect("migrate legacy source");

    // The subset boundary is exactly what we expect: six in-subset declarations,
    // and the four out-of-subset ones named with their typed reasons.
    assert_eq!(
        migration.migrated_names(),
        MIGRATED,
        "migrated declarations, in schema order",
    );

    let mut excluded: Vec<(&str, &str)> = migration
        .excluded
        .iter()
        .map(|exclusion| {
            let kind = match exclusion.reason {
                BridgeError::UnsupportedApplication(_) => "application",
                BridgeError::UnsupportedEnum(_) => "enum",
                BridgeError::UnsupportedScalar(_) => "scalar",
                BridgeError::UnsupportedText(_) => "text",
                BridgeError::UnsupportedInlineField(_) => "inline-field",
                BridgeError::LegacyParse(_) | BridgeError::NameTable(_) => "other",
            };
            (exclusion.name.as_str(), kind)
        })
        .collect();
    excluded.sort();
    assert_eq!(
        excluded,
        [
            ("Kind", "enum"),
            ("Magnitude", "enum"),
            ("RecordSet", "application"),
            ("Topics", "application"),
        ],
        "out-of-subset declarations, named with their typed reasons",
    );

    // new CoreSchema -> Nomos macros -> CoreLogos -> TextualRust -> Rust.
    let lowering = MacroPackage::wire_fixture()
        .apply(&migration.schema, &migration.names)
        .expect("lower migrated schema");

    // Each migrated item must reproduce its legacy golden bytes exactly.
    let goldens = golden_item_blocks(LEGACY_GOLDEN);
    let golden_of = |name: &str| -> String {
        goldens
            .iter()
            .find(|(candidate, _)| candidate == name)
            .unwrap_or_else(|| panic!("golden block for {name}"))
            .1
            .clone()
    };

    assert_eq!(
        lowering.items.len(),
        MIGRATED.len(),
        "one item per migration"
    );
    for (item, expected_name) in lowering.items.iter().zip(MIGRATED) {
        let rust = project(item, &lowering.names);
        assert_eq!(
            rust,
            golden_of(expected_name),
            "byte-exact migration of {expected_name}",
        );
    }
}
