#![allow(unused)]

mod names;
mod relations;
mod types;

use self::relations::RelationField;
use crate::{
    ast::{self, FieldId, TopId},
    diagnostics::{DatamodelError, Diagnostics},
};
use names::Names;
use std::str::FromStr;

/// Information gathered during schema validation. Each validation steps
/// enriches the database with information that can be used in later steps.
pub(crate) struct ParserDatabase<'a> {
    ast: &'a ast::SchemaAst,
    names: Names<'a>,
    types: types::Types,
    relations: relations::Relations,
}

impl<'ast> ParserDatabase<'ast> {
    pub(super) fn new(ast: &'ast ast::SchemaAst, diagnostics: &mut Diagnostics) -> Option<Self> {
        let names = Names::new(ast, diagnostics);

        if diagnostics.has_errors() {
            return None;
        }

        let mut types = types::Types::default();
        let mut relations = relations::Relations::default();

        for (top_id, top) in ast.iter_tops() {
            match top {
                ast::Top::Type(type_alias) => {
                    match field_type(type_alias, &names, ast) {
                        Ok(FieldType::Scalar(scalar_field_type)) => {
                            types.type_aliases.insert(top_id, scalar_field_type);
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
                ast::Top::Model(model) => {
                    for (field_id, field) in model.iter_fields() {
                        match field_type(field, &names, ast) {
                            Ok(FieldType::Model(referenced_model)) => {
                                relations
                                    .relation_fields
                                    .insert((top_id, field_id), relations::RelationField { referenced_model });
                            }
                            Ok(FieldType::Scalar(scalar_field_type)) => {
                                types.scalar_fields.insert((top_id, field_id), scalar_field_type);
                            }
                            Err(supported) => diagnostics.push_error(DatamodelError::new_type_not_found_error(
                                supported,
                                field.field_type.span(),
                            )),
                        }
                    }
                }
                ast::Top::Source(_) | ast::Top::Generator(_) | ast::Top::Enum(_) => (),
            }
        }

        types.detect_alias_cycles(ast, diagnostics);

        Some(ParserDatabase {
            ast,
            names,
            types,
            relations,
        })
    }

    pub(super) fn ast(&self) -> &'ast ast::SchemaAst {
        self.ast
    }

    pub(crate) fn iter_enums(&self) -> impl Iterator<Item = (TopId, &'ast ast::Enum)> + '_ {
        self.names
            .tops
            .values()
            .filter_map(move |topid| self.ast[*topid].as_enum().map(|enm| (*topid, enm)))
    }

    pub(crate) fn iter_model_relation_fields(
        &self,
        top_id: TopId,
    ) -> impl Iterator<Item = (FieldId, &RelationField)> + '_ {
        self.relations
            .relation_fields
            .range((top_id, FieldId::ZERO)..=(top_id, FieldId::MAX))
            .map(|((_, field_id), rf)| (*field_id, rf))
    }

    pub(crate) fn iter_model_scalar_fields(
        &self,
        model_id: TopId,
    ) -> impl Iterator<Item = (FieldId, &ScalarFieldType)> + '_ {
        self.types
            .scalar_fields
            .range((model_id, FieldId::ZERO)..=(model_id, FieldId::MAX))
            .map(|((_, field_id), scalar_type)| (*field_id, scalar_type))
    }

    pub(super) fn get_enum(&self, name: &str) -> Option<&'ast ast::Enum> {
        self.names.tops.get(name).and_then(|top_id| self.ast[*top_id].as_enum())
    }
}

#[derive(Debug)]
enum FieldType {
    Model(TopId),
    Scalar(ScalarFieldType),
}

#[derive(Debug)]
pub(crate) enum ScalarFieldType {
    Enum(TopId),
    BuiltInScalar,
    Alias(TopId),
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
