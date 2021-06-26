#![deny(missing_docs)]

//! See the docs on [ParserDatabase](/struct.ParserDatabase.html).

mod names;
mod relations;
mod types;

use self::relations::RelationField;
use crate::{
    ast,
    diagnostics::{DatamodelError, Diagnostics},
    Datasource,
};
use datamodel_connector::{Connector, EmptyDatamodelConnector};
use names::Names;
use std::str::FromStr;

/// ParserDatabase is a container for a Schema AST, together with information
/// gathered during schema validation. Each validation step enriches the
/// database with information that can be used to work with the schema, without
/// changing the AST. Instantiating with `ParserDatabase::new()` will performa a
/// number of validations and make sure the schema makes sense, but it cannot
/// fail. In case the schema is invalid, diagnostics will be created and the
/// resolved information will be incomplete.
pub(crate) struct ParserDatabase<'a> {
    ast: &'a ast::SchemaAst,
    datasource: Option<&'a Datasource>,
    names: Names<'a>,
    types: types::Types,
    relations: relations::Relations,
}

impl<'ast> ParserDatabase<'ast> {
    /// See the docs on [ParserDatabase](/struct.ParserDatabase.html).
    pub(super) fn new(
        ast: &'ast ast::SchemaAst,
        datasource: Option<&'ast Datasource>,
        diagnostics: &mut Diagnostics,
    ) -> Self {
        let mut db = ParserDatabase {
            ast,
            datasource,
            names: Names::new(ast, diagnostics),
            types: types::Types::default(),
            relations: relations::Relations::default(),
        };

        // Abort early on name resolution errors.
        if diagnostics.has_errors() {
            return db;
        }

        for (top_id, top) in ast.iter_tops() {
            match top {
                ast::Top::Type(type_alias) => {
                    match field_type(type_alias, &db.names, ast) {
                        Ok(FieldType::Scalar(scalar_field_type)) => {
                            db.types.type_aliases.insert(top_id, scalar_field_type);
                        }
                        Ok(FieldType::Model(_)) => diagnostics.push_error(DatamodelError::new_validation_error(
                            "Only scalar types can be used for defining custom types.",
                            type_alias.field_type.span(),
                        )),
                        Err(supported) => diagnostics.push_error(DatamodelError::new_type_not_found_error(
                            supported,
                            type_alias.field_type.span(),
                        )),
                    };
                }
                ast::Top::Model(model) => db.visit_model(top_id, model, ast, diagnostics),
                ast::Top::Source(_) | ast::Top::Generator(_) | ast::Top::Enum(_) => (),
            }
        }

        db.types.detect_alias_cycles(ast, diagnostics);

        db
    }

    pub(super) fn alias_scalar_field_type(&self, alias_id: &ast::TopId) -> &ScalarFieldType {
        self.types.type_aliases.get(alias_id).unwrap()
    }

    pub(super) fn ast(&self) -> &'ast ast::SchemaAst {
        self.ast
    }

    pub(super) fn datasource(&self) -> Option<&'ast Datasource> {
        self.datasource
    }

    pub(super) fn active_connector(&self) -> &dyn Connector {
        self.datasource
            .map(|datasource| datasource.active_connector.as_ref())
            .unwrap_or(&EmptyDatamodelConnector)
    }

    pub(crate) fn iter_enums(&self) -> impl Iterator<Item = (ast::TopId, &'ast ast::Enum)> + '_ {
        self.names
            .tops
            .values()
            .filter_map(move |topid| self.ast[*topid].as_enum().map(|enm| (*topid, enm)))
    }

    /// Iterate all the relation fields in a given model in the order they were
    /// defined. Note that these are the fields that were actually written in
    /// the schema.
    pub(crate) fn iter_model_relation_fields(
        &self,
        model_id: ast::TopId,
    ) -> impl Iterator<Item = (ast::FieldId, &RelationField)> + '_ {
        self.relations
            .relation_fields
            .range((model_id, ast::FieldId::ZERO)..=(model_id, ast::FieldId::MAX))
            .map(|((_, field_id), rf)| (*field_id, rf))
    }

    /// Iterate all the scalar fields in a given model in the order they were defined.
    pub(crate) fn iter_model_scalar_fields(
        &self,
        model_id: ast::TopId,
    ) -> impl Iterator<Item = (ast::FieldId, &ScalarFieldType)> + '_ {
        self.types
            .scalar_fields
            .range((model_id, ast::FieldId::ZERO)..=(model_id, ast::FieldId::MAX))
            .map(|((_, field_id), scalar_type)| (*field_id, scalar_type))
    }

    pub(super) fn get_enum(&self, name: &str) -> Option<&'ast ast::Enum> {
        self.names.tops.get(name).and_then(|top_id| self.ast[*top_id].as_enum())
    }

    fn visit_model(
        &mut self,
        top_id: ast::TopId,
        model: &ast::Model,
        ast: &ast::SchemaAst,
        diagnostics: &mut Diagnostics,
    ) {
        for (field_id, field) in model.iter_fields() {
            match field_type(field, &self.names, ast) {
                Ok(FieldType::Model(referenced_model)) => {
                    self.relations
                        .relation_fields
                        .insert((top_id, field_id), relations::RelationField { referenced_model });
                }
                Ok(FieldType::Scalar(scalar_field_type)) => {
                    self.types.scalar_fields.insert((top_id, field_id), scalar_field_type);
                }
                Err(supported) => diagnostics.push_error(DatamodelError::new_type_not_found_error(
                    supported,
                    field.field_type.span(),
                )),
            }
        }
    }
}

#[derive(Debug)]
enum FieldType {
    Model(ast::TopId),
    Scalar(ScalarFieldType),
}

#[derive(Debug)]
pub(crate) enum ScalarFieldType {
    Enum(ast::TopId),
    BuiltInScalar,
    Alias(ast::TopId),
    Unsupported,
}

fn field_type<'a>(field: &'a ast::Field, names: &Names<'_>, ast: &'a ast::SchemaAst) -> Result<FieldType, &'a str> {
    let supported = match &field.field_type {
        ast::FieldType::Supported(ident) => &ident.name,
        ast::FieldType::Unsupported(_, _) => return Ok(FieldType::Scalar(ScalarFieldType::Unsupported)),
    };

    if dml::scalars::ScalarType::from_str(supported).is_ok() {
        return Ok(FieldType::Scalar(ScalarFieldType::BuiltInScalar));
    }

    match names.tops.get(supported.as_str()).map(|id| (*id, &ast[*id])) {
        Some((id, ast::Top::Model(_))) => Ok(FieldType::Model(id)),
        Some((id, ast::Top::Enum(_))) => Ok(FieldType::Scalar(ScalarFieldType::Enum(id))),
        Some((id, ast::Top::Type(_))) => Ok(FieldType::Scalar(ScalarFieldType::Alias(id))),
        Some((_, ast::Top::Generator(_))) | Some((_, ast::Top::Source(_))) => unreachable!(),
        None => Err(supported),
    }
}
