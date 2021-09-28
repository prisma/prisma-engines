#![deny(missing_docs)]

//! See the docs on [ParserDatabase](/struct.ParserDatabase.html).

mod attributes;
mod context;
mod names;
mod relations;
mod types;
mod walkers;

pub(crate) use types::{ScalarField, ScalarFieldType};

use self::{
    context::Context,
    relations::Relations,
    types::{RelationField, Types},
    walkers::ExplicitRelationWalker,
};
use crate::PreviewFeature;
use crate::{ast, diagnostics::Diagnostics, Datasource};
use datamodel_connector::{Connector, EmptyDatamodelConnector};
use enumflags2::BitFlags;
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
pub(crate) struct ParserDatabase<'ast> {
    ast: &'ast ast::SchemaAst,
    datasource: Option<&'ast Datasource>,
    names: Names<'ast>,
    types: Types<'ast>,
    relations: Relations<'ast>,
    _preview_features: BitFlags<PreviewFeature>,
}

impl<'ast> ParserDatabase<'ast> {
    /// See the docs on [ParserDatabase](/struct.ParserDatabase.html).
    pub(super) fn new(
        ast: &'ast ast::SchemaAst,
        datasource: Option<&'ast Datasource>,
        diagnostics: Diagnostics,
        preview_features: BitFlags<PreviewFeature>,
    ) -> (Self, Diagnostics) {
        let db = ParserDatabase {
            ast,
            datasource,
            names: Names::default(),
            types: Types::default(),
            relations: Relations::default(),
            _preview_features: preview_features,
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

        // Fourth step: global validations
        attributes::validate_index_names(&mut ctx);
        attributes::fill_in_default_constraint_names(&mut ctx);
        attributes::validate_relation_fields(&mut ctx);

        // Fifth step: relation inference
        relations::infer_relations(&mut ctx);
        relations::validate_relations(&mut ctx);

        ctx.finish()
    }

    pub(super) fn alias_scalar_field_type(&self, alias_id: &ast::AliasId) -> &ScalarFieldType {
        &self.types.type_aliases[alias_id]
    }

    pub(super) fn ast(&self) -> &'ast ast::SchemaAst {
        self.ast
    }

    pub(super) fn datasource(&self) -> Option<&'ast Datasource> {
        self.datasource
    }

    pub(crate) fn find_model_field(&self, model_id: ast::ModelId, field_name: &str) -> Option<ast::FieldId> {
        self.names.model_fields.get(&(model_id, field_name)).cloned()
    }

    pub(crate) fn get_enum_database_name(&self, enum_id: ast::EnumId) -> Option<&'ast str> {
        self.types.enum_attributes[&enum_id].mapped_name
    }

    pub(crate) fn get_enum_value_database_name(&self, enum_id: ast::EnumId, value_idx: u32) -> Option<&'ast str> {
        self.types.enum_attributes[&enum_id]
            .mapped_values
            .get(&value_idx)
            .cloned()
    }

    pub(super) fn active_connector(&self) -> &dyn Connector {
        self.datasource
            .map(|datasource| datasource.active_connector.as_ref())
            .unwrap_or(&EmptyDatamodelConnector)
    }

    /// Iterate all the relation fields in a given model in the order they were
    /// defined. Note that these are only the fields that were actually written
    /// in the schema.
    fn iter_model_relation_fields(
        &self,
        model_id: ast::ModelId,
    ) -> impl Iterator<Item = (ast::FieldId, &RelationField<'ast>)> + '_ {
        self.types
            .relation_fields
            .range((model_id, ast::FieldId::ZERO)..=(model_id, ast::FieldId::MAX))
            .map(|((_, field_id), rf)| (*field_id, rf))
    }

    /// Iterate all the scalar fields in a given model in the order they were defined.
    pub(crate) fn iter_model_scalar_fields(
        &self,
        model_id: ast::ModelId,
    ) -> impl Iterator<Item = (ast::FieldId, &ScalarField<'ast>)> {
        self.types
            .scalar_fields
            .range((model_id, ast::FieldId::ZERO)..=(model_id, ast::FieldId::MAX))
            .map(|((_, field_id), scalar_type)| (*field_id, scalar_type))
    }

    /// Iterate all complete relations that are not many to many.
    pub(crate) fn walk_explicit_relations(&self) -> impl Iterator<Item = ExplicitRelationWalker<'ast, '_>> + '_ {
        self.relations
            .iter_relations()
            .filter(|(_, _, relation)| !relation.is_many_to_many())
            .filter_map(move |(model_a, model_b, relation)| {
                relation
                    .as_complete_fields()
                    .map(|(field_a, field_b)| ExplicitRelationWalker {
                        side_a: (model_a, field_a),
                        side_b: (model_b, field_b),
                        db: self,
                        relation,
                    })
            })
    }
}
