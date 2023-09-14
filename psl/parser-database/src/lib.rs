#![deny(unsafe_code, rust_2018_idioms, missing_docs)]
#![allow(clippy::derive_partial_eq_without_eq)]

//! See the docs on [ParserDatabase](./struct.ParserDatabase.html).
//!
//! ## Scope
//!
//! The ParserDatabase is tasked with gathering information about the schema. It is _connector
//! agnostic_: it gathers information and performs generic validations, leaving connector-specific
//! validations to later phases in datamodel core.
//!
//! ## Terminology
//!
//! Names:
//!
//! - _name_: the item name in the schema for datasources, generators, models, model fields,
//!   composite types, composite type fields, enums and enum variants. The `name:` argument for
//!   unique constraints, primary keys and relations.
//! - _mapped name_: the name inside an `@map()` or `@@map()` attribute of a model, field, enum or
//!   enum value. This is used to determine what the name of the Prisma schema item is in the
//!   database.
//! - _database name_: the name in the database, once both the name of the item and the mapped
//!   name have been taken into account. The logic is always the same: if a mapped name is defined,
//!   then the database name is the mapped name, otherwise it is the name of the item.
//! - _constraint name_: indexes, primary keys and defaults can have a constraint name. It can be
//!   defined with a `map:` argument or be a default, generated name if the `map:` argument is not
//!   provided. These usually require a datamodel connector to be defined.

pub mod walkers;

mod attributes;
mod coerce_expression;
mod context;
mod files;
mod ids;
mod interner;
mod names;
mod relations;
mod types;

pub use coerce_expression::{coerce, coerce_array, coerce_opt};
pub use ids::*;
pub use names::is_reserved_type_name;
pub use relations::{ManyToManyRelationId, ReferentialAction, RelationId};
pub use schema_ast::{ast, SourceFile};
pub use types::{
    IndexAlgorithm, IndexFieldPath, IndexType, OperatorClass, RelationFieldId, ScalarFieldId, ScalarFieldType,
    ScalarType, SortOrder,
};

use self::{context::Context, files::Files, interner::StringId, relations::Relations, types::Types};
use diagnostics::{DatamodelError, Diagnostics};
use names::Names;
use std::collections::HashMap;

/// ParserDatabase is a container for a Schema AST, together with information
/// gathered during schema validation. Each validation step enriches the
/// database with information that can be used to work with the schema, without
/// changing the AST. Instantiating with `ParserDatabase::new()` will perform a
/// number of validations and make sure the schema makes sense, but it cannot
/// fail. In case the schema is invalid, diagnostics will be created and the
/// resolved information will be incomplete.
///
/// Validations are carried out in the following order:
///
/// - The AST is walked a first time to resolve names: to each relevant
///   identifier, we attach an ID that can be used to reference the
///   corresponding item (model, enum, field, ...)
/// - The AST is walked a second time to resolve types. For each field and each
///   type alias, we look at the type identifier and resolve what it refers to.
/// - The AST is walked a third time to validate attributes on models and
///   fields.
/// - Global validations are then performed on the mostly validated schema.
///   Currently only index name collisions.
pub struct ParserDatabase {
    schemas: Vec<(String, schema_ast::SourceFile)>,
    asts: Files, // todo: combine schemas and ASTs
    interner: interner::StringInterner,
    names: Names,
    types: Types,
    relations: Relations,
}

impl ParserDatabase {
    /// See the docs on [ParserDatabase](/struct.ParserDatabase.html).
    pub fn new(schemas: Vec<(String, schema_ast::SourceFile)>, diagnostics: &mut Diagnostics) -> Self {
        let asts: HashMap<_, _> = schemas
            .iter()
            .enumerate()
            .map(|(schema_idx, (_path, schema))| {
                (
                    SchemaId(schema_idx),
                    schema_ast::parse_schema(schema.as_str(), diagnostics),
                )
            })
            .collect();
        let asts = Files(asts);

        let mut interner = Default::default();
        let mut names = Default::default();
        let mut types = Default::default();
        let mut relations = Default::default();
        let mut ctx = Context::new(
            &asts,
            &mut interner,
            &mut names,
            &mut types,
            &mut relations,
            diagnostics,
        );

        // First pass: resolve names.
        names::resolve_names(&mut ctx);

        // Return early on name resolution errors.
        if ctx.diagnostics.has_errors() {
            attributes::create_default_attributes(&mut ctx);

            return ParserDatabase {
                asts,
                schemas,
                interner,
                names,
                types,
                relations,
            };
        }

        // Second pass: resolve top-level items and field types.
        types::resolve_types(&mut ctx);

        // Return early on type resolution errors.
        if ctx.diagnostics.has_errors() {
            attributes::create_default_attributes(&mut ctx);

            return ParserDatabase {
                asts,
                schemas,
                interner,
                names,
                types,
                relations,
            };
        }

        // Third pass: validate model and field attributes. All these
        // validations should be _order independent_ and only rely on
        // information from previous steps, not from other attributes.
        attributes::resolve_attributes(&mut ctx);

        // Fourth step: relation inference
        relations::infer_relations(&mut ctx);

        ParserDatabase {
            asts,
            schemas,
            interner,
            names,
            types,
            relations,
        }
    }

    /// A parsed AST.
    pub fn ast(&self, schema_id: SchemaId) -> &ast::SchemaAst {
        &self.asts[schema_id]
    }

    /// The total number of enums in the schema. This is O(1).
    pub fn enums_count(&self) -> usize {
        self.types.enum_attributes.len()
    }

    /// The total number of models in the schema. This is O(1).
    pub fn models_count(&self) -> usize {
        self.types.model_attributes.len()
    }

    /// The source file contents.
    pub fn source(&self, schema_id: SchemaId) -> &str {
        self.schemas[schema_id.0].1.as_str()
    }
}

impl std::fmt::Debug for ParserDatabase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ParserDatabase { ... }")
    }
}

impl std::ops::Index<StringId> for ParserDatabase {
    type Output = str;

    fn index(&self, index: StringId) -> &Self::Output {
        self.interner.get(index).unwrap()
    }
}
