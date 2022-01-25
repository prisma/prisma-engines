#![deny(unsafe_code, rust_2018_idioms, missing_docs)]

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

mod ast_string;
mod attributes;
mod context;
mod indexes;
mod names;
mod relations;
mod types;
mod value_validator;

pub use names::is_reserved_type_name;
pub use relations::ReferentialAction;
pub use schema_ast::ast;
pub use types::{IndexAlgorithm, IndexType, ScalarFieldType, ScalarType, SortOrder};
pub use value_validator::{ValueListValidator, ValueValidator};

use self::{ast_string::AstString, context::Context, relations::Relations, types::Types};
use diagnostics::{DatamodelError, Diagnostics};
use names::Names;

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
///
/// ## Lifetimes
///
/// Throughout the ParserDatabase implementation, you will see many lifetime
/// annotations. The only significant lifetime is the lifetime of the reference
/// to the AST contained in ParserDatabase, that we call by convention `'ast`.
/// Apart from that, everything should be owned or locally borrowed, to keep
/// lifetime management simple.
pub struct ParserDatabase<'ast> {
    src: &'ast str,
    ast: &'ast ast::SchemaAst,
    names: Names<'ast>,
    types: Types<'ast>,
    relations: Relations,
}

impl<'ast> ParserDatabase<'ast> {
    /// See the docs on [ParserDatabase](/struct.ParserDatabase.html).
    pub fn new(src: &'ast str, ast: &'ast ast::SchemaAst, diagnostics: Diagnostics) -> (Self, Diagnostics) {
        let db = ParserDatabase {
            ast,
            src,
            names: Names::default(),
            types: Types::default(),
            relations: Relations::default(),
        };

        let mut ctx = Context::new(db, diagnostics);

        // First pass: resolve names.
        names::resolve_names(&mut ctx);

        // Return early on name resolution errors.
        if ctx.has_errors() {
            return ctx.finish();
        }

        // Second pass: resolve top-level items and field types.
        types::resolve_types(&mut ctx);

        // Return early on type resolution errors.
        if ctx.has_errors() {
            return ctx.finish();
        }

        // Third pass: validate model and field attributes. All these
        // validations should be _order independent_ and only rely on
        // information from previous steps, not from other attributes.
        attributes::resolve_attributes(&mut ctx);

        // Fourth step: relation inference
        relations::infer_relations(&mut ctx);

        // Fifth step: infer implicit indices
        indexes::infer_implicit_indexes(&mut ctx);

        ctx.finish()
    }

    /// The fully resolved (non alias) scalar field type of an alias. .
    pub fn alias_scalar_field_type(&self, alias_id: &ast::AliasId) -> &ScalarFieldType {
        &self.types.type_aliases[alias_id]
    }

    /// The parsed AST.
    pub fn ast(&self) -> &'ast ast::SchemaAst {
        self.ast
    }

    pub(crate) fn resolve_str<'a>(&'a self, s: &'a AstString) -> &str {
        if let Some(unescaped) = &s.unescaped {
            unescaped
        } else {
            &self.src[s.span.start..s.span.end]
        }
    }

    /// Find a specific field in a specific model.
    fn find_model_field(&self, model_id: ast::ModelId, field_name: &str) -> Option<ast::FieldId> {
        self.names.model_fields.get(&(model_id, field_name)).cloned()
    }
}

impl std::fmt::Debug for ParserDatabase<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ParserDatabase { ... }")
    }
}
