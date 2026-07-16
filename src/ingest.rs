//! The legacy-source ingestion: a real decoder from legacy `schema-language` text
//! into the new stringless `CoreSchema` + `NameTable` model.
//!
//! Parsing is the legacy parser's job ([`SchemaSource::from_schema_text`]); this
//! module only translates the already-parsed legacy model, object to object, into
//! the new Core. It carries no strings into the Core: every legacy name is
//! re-interned into the new [`NameTable`] and the declarations hold only
//! [`Identifier`]s.
//!
//! Leg-one subset: a newtype over a scalar or plain reference, and a struct whose
//! fields are scalar or plain references (elided or explicit). Every construct
//! beyond that — generic applications, enums, pipe-text, inline field
//! declarations, the legacy `Path` scalar — is a loud [`BridgeError`], collected as
//! a named exclusion rather than guessed at. The legacy schema surface carries no
//! visibility token, so every migrated declaration is public API intent, which the
//! downstream Nomos lowering then honours faithfully.

use core_schema::{
    CoreDeclaration, CoreField, CoreNewtype, CoreReference, CoreSchema, CoreStruct, CoreType,
    Visibility,
};
use name_table::{Identifier, Name, NameTable};
use schema_language::{
    SchemaSource, SourceDeclarationValue, SourceField, SourceFieldValue, SourceReference,
    SourceStructBody, SourceTypeEntry,
};

use crate::error::BridgeError;

/// One legacy declaration the leg-one subset could not carry, named with the exact
/// typed reason it was excluded. The migration keeps these so the out-of-subset
/// frontier is always visible, never a silent drop.
#[derive(Debug)]
pub struct Exclusion {
    /// The legacy declaration's type name.
    pub name: String,
    /// Why the leg-one subset could not migrate it.
    pub reason: BridgeError,
}

/// The result of migrating a legacy schema source: the new `CoreSchema` holding
/// only the migrated (in-subset) declarations, the `NameTable` every identifier in
/// it resolves through, and the named exclusions for everything beyond the subset.
#[derive(Debug)]
pub struct Migration {
    /// The migrated declarations as a stringless `CoreSchema`.
    pub schema: CoreSchema,
    /// The table every identifier in `schema` resolves through, including names of
    /// referenced-but-excluded types (a struct may reference a type the subset does
    /// not itself migrate).
    pub names: NameTable,
    /// The out-of-subset declarations, each with its typed reason.
    pub excluded: Vec<Exclusion>,
}

impl Migration {
    /// The names of the declarations that migrated into the Core, in schema order.
    pub fn migrated_names(&self) -> Vec<String> {
        self.schema
            .declarations()
            .iter()
            .map(|declaration| {
                self.names
                    .resolve(declaration.identifier())
                    .map(|name| name.as_str().to_owned())
                    .unwrap_or_default()
            })
            .collect()
    }
}

/// Ingests a legacy `schema-language` source into the new Core, accumulating the
/// shared `NameTable` as it goes. Stateful because interning is: the same legacy
/// name always resolves to the same new `Identifier`.
pub struct LegacySchemaIngest {
    names: NameTable,
}

impl Default for LegacySchemaIngest {
    fn default() -> Self {
        Self::new()
    }
}

impl LegacySchemaIngest {
    /// A fresh ingest over an empty table.
    pub fn new() -> Self {
        Self {
            names: NameTable::new(),
        }
    }

    /// Parse legacy schema source text and migrate its whole types block into a
    /// [`Migration`]. Parse failures are fatal; per-declaration subset failures are
    /// collected as named exclusions.
    pub fn migrate_text(text: &str) -> Result<Migration, BridgeError> {
        let source = SchemaSource::from_schema_text(text)?;
        Ok(Self::new().migrate_source(&source))
    }

    /// Migrate the types block of an already-parsed legacy source.
    pub fn migrate_source(mut self, source: &SchemaSource) -> Migration {
        let mut declarations = Vec::new();
        let mut excluded = Vec::new();
        for entry in source.types().entries() {
            match self.migrate_entry(entry) {
                Ok(declaration) => declarations.push(declaration),
                Err(reason) => excluded.push(Exclusion {
                    name: entry.name().as_str().to_owned(),
                    reason,
                }),
            }
        }
        Migration {
            schema: CoreSchema::new(declarations),
            names: self.names,
            excluded,
        }
    }

    /// Migrate one legacy type declaration. The legacy schema surface carries no
    /// visibility, so every migrated declaration is public — the authoritative
    /// public-API intent the Nomos lowering later stamps onto the item.
    fn migrate_entry(&mut self, entry: &SourceTypeEntry) -> Result<CoreDeclaration, BridgeError> {
        let name = entry.name();
        let value = match entry.value() {
            SourceDeclarationValue::Reference(reference) => {
                let wrapped = self.migrate_reference(reference, name.as_str())?;
                CoreType::Newtype(CoreNewtype::new(self.intern(name.as_str()), wrapped))
            }
            SourceDeclarationValue::Struct(body) => {
                // Legacy rule (schema-language `source.rs:1858`): a one-field brace
                // body is a newtype wrapping that field's reference, not a
                // single-field struct — the same convention the new grammar uses,
                // and what the golden emits. Two-or-more fields is a named struct.
                if let [single] = body.fields() {
                    let wrapped = self.field_reference(single, name.as_str())?;
                    CoreType::Newtype(CoreNewtype::new(self.intern(name.as_str()), wrapped))
                } else {
                    let fields = self.migrate_fields(body, name.as_str())?;
                    CoreType::Struct(CoreStruct::new(self.intern(name.as_str()), fields))
                }
            }
            SourceDeclarationValue::Enum(_) => {
                return Err(BridgeError::UnsupportedEnum(name.as_str().to_owned()));
            }
            SourceDeclarationValue::Text(_) => {
                return Err(BridgeError::UnsupportedText(name.as_str().to_owned()));
            }
        };
        Ok(CoreDeclaration::new(Visibility::Public, value))
    }

    /// Migrate a struct body's fields.
    fn migrate_fields(
        &mut self,
        body: &SourceStructBody,
        owner: &str,
    ) -> Result<Vec<CoreField>, BridgeError> {
        body.fields()
            .iter()
            .map(|field| self.migrate_field(field, owner))
            .collect()
    }

    /// Migrate one struct field. An elided (derived) field defers its stored name to
    /// the reference's single-homed `derived_field_name`, so the elision predicate
    /// re-elides it downstream; an explicit field keeps its own name verbatim.
    fn migrate_field(
        &mut self,
        field: &SourceField,
        owner: &str,
    ) -> Result<CoreField, BridgeError> {
        let reference = self.field_reference(field, owner)?;
        let identifier = if let SourceFieldValue::Reference(_) = field.value() {
            self.intern(field.name().as_str())
        } else {
            let derived = reference.derived_field_name(&self.names)?;
            self.names.intern(Name::new(derived))
        };
        Ok(CoreField::new(identifier, reference))
    }

    /// The new reference a legacy field carries, ignoring its name. Shared by the
    /// struct-field path and the single-field-newtype path. An inline nested
    /// declaration is out of the leg-one subset.
    fn field_reference(
        &mut self,
        field: &SourceField,
        owner: &str,
    ) -> Result<CoreReference, BridgeError> {
        match field.value() {
            SourceFieldValue::Derived => self.migrate_plain(field.name().as_str()),
            SourceFieldValue::Reference(reference) => self.migrate_reference(reference, owner),
            SourceFieldValue::Declaration(_) => {
                Err(BridgeError::UnsupportedInlineField(owner.to_owned()))
            }
        }
    }

    /// Migrate a legacy reference. Only a plain reference is in the leg-one subset;
    /// every generic application is named and excluded.
    fn migrate_reference(
        &mut self,
        reference: &SourceReference,
        context: &str,
    ) -> Result<CoreReference, BridgeError> {
        match reference {
            SourceReference::Plain(name) => self.migrate_plain(name.as_str()),
            SourceReference::ValueApplication(_)
            | SourceReference::SingleTypeApplication(_)
            | SourceReference::MultiTypeApplication(_)
            | SourceReference::Application { .. } => {
                Err(BridgeError::UnsupportedApplication(context.to_owned()))
            }
        }
    }

    /// Classify a plain legacy type name: a scalar keyword becomes the matching new
    /// leaf, the legacy `Path` scalar is a loud exclusion (the new Core has no such
    /// leaf), and any other name is a `Plain` reference to a declared type, carrying
    /// the identifier its name interns to.
    fn migrate_plain(&mut self, spelling: &str) -> Result<CoreReference, BridgeError> {
        Ok(match spelling {
            "String" => CoreReference::String,
            "Integer" => CoreReference::Integer,
            "Boolean" => CoreReference::Boolean,
            "Bytes" => CoreReference::Bytes,
            "Path" => return Err(BridgeError::UnsupportedScalar("Path".to_owned())),
            _ => CoreReference::Plain(self.intern(spelling)),
        })
    }

    /// Intern a legacy name spelling into the new table.
    fn intern(&mut self, spelling: &str) -> Identifier {
        self.names.intern(Name::new(spelling))
    }
}
