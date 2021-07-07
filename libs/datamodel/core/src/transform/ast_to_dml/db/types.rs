use super::{attributes, context::Context};
use crate::{ast, diagnostics::DatamodelError};
use itertools::Itertools;
use std::{
    collections::{BTreeMap, HashMap},
    str::FromStr,
};

pub(super) fn resolve_types(ctx: &mut Context<'_>) {
    for (top_id, top) in ctx.db.ast.iter_tops() {
        match (top_id, top) {
            (ast::TopId::Alias(alias_id), ast::Top::Type(type_alias)) => visit_type_alias(alias_id, type_alias, ctx),
            (ast::TopId::Model(model_id), ast::Top::Model(model)) => visit_model(model_id, model, ctx),
            (ast::TopId::Enum(enum_id), ast::Top::Enum(enm)) => visit_enum(enum_id, enm, ctx),
            (_, ast::Top::Source(_)) | (_, ast::Top::Generator(_)) => (),
            _ => unreachable!(),
        }
    }

    detect_alias_cycles(ctx);
}

#[derive(Debug, Default)]
pub(super) struct Types<'ast> {
    pub(super) type_aliases: HashMap<ast::AliasId, ScalarFieldType>,
    pub(super) scalar_fields: BTreeMap<(ast::ModelId, ast::FieldId), ScalarField<'ast>>,
    pub(super) models: HashMap<ast::ModelId, ModelData<'ast>>,
    /// This contains only the relation fields actually present in the schema
    /// source text.
    pub(super) relation_fields: BTreeMap<(ast::ModelId, ast::FieldId), RelationField<'ast>>,
    pub(super) enums: HashMap<ast::EnumId, EnumData<'ast>>,
}

impl<'ast> Types<'ast> {
    pub(super) fn take_model_data(&mut self, model_id: &ast::ModelId) -> Option<ModelData<'ast>> {
        self.models.remove(model_id)
    }

    pub(super) fn take_scalar_field(
        &mut self,
        model_id: ast::ModelId,
        field_id: ast::FieldId,
    ) -> Option<ScalarField<'ast>> {
        self.scalar_fields.remove(&(model_id, field_id))
    }

    pub(super) fn take_relation_field(
        &mut self,
        model_id: ast::ModelId,
        field_id: ast::FieldId,
    ) -> Option<RelationField<'ast>> {
        self.relation_fields.remove(&(model_id, field_id))
    }
}

#[derive(Debug)]
enum FieldType {
    Model(ast::ModelId),
    Scalar(ScalarFieldType),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ScalarFieldType {
    Enum(ast::EnumId),
    BuiltInScalar(dml::scalars::ScalarType),
    Alias(ast::AliasId),
    Unsupported,
}

#[derive(Debug)]
pub(crate) struct ScalarField<'ast> {
    pub(crate) r#type: ScalarFieldType,
    pub(crate) is_ignored: bool,
    pub(crate) is_updated_at: bool,
    pub(crate) default: Option<dml::default_value::DefaultValue>,
    /// @map
    pub(super) mapped_name: Option<&'ast str>,
}

#[derive(Debug)]
pub(crate) struct RelationField<'ast> {
    pub(crate) referenced_model: ast::ModelId,
    pub(crate) on_delete: Option<dml::relation_info::ReferentialAction>,
    pub(crate) on_update: Option<dml::relation_info::ReferentialAction>,
    /// The fields _explicitly present_ in the AST.
    pub(crate) fields: Option<Vec<ast::FieldId>>,
    /// The `references` fields _explicitly present_ in the AST.
    pub(crate) references: Option<Vec<ast::FieldId>>,
    /// The name _explicitly present_ in the AST.
    pub(crate) name: Option<&'ast str>,
    pub(crate) is_ignored: bool,
}

impl RelationField<'_> {
    fn new(referenced_model: ast::ModelId) -> Self {
        RelationField {
            referenced_model,
            on_delete: None,
            on_update: None,
            fields: None,
            references: None,
            name: None,
            is_ignored: false,
        }
    }
}

#[derive(Default, Debug)]
pub(crate) struct ModelData<'ast> {
    pub(crate) id_fields: Option<Vec<ast::FieldId>>,
    // When the id came from an @id on a single field.
    pub(crate) id_source_field: Option<ast::FieldId>,
    pub(crate) is_ignored: bool,
    /// @(@) index and @(@)unique.
    pub(crate) indexes: Vec<IndexData<'ast>>,
    /// @@map
    pub(super) mapped_name: Option<&'ast str>,
}

#[derive(Debug, Default)]
pub(crate) struct IndexData<'ast> {
    pub(crate) is_unique: bool,
    pub(crate) fields: Vec<ast::FieldId>,
    pub(crate) source_field: Option<ast::FieldId>,
    pub(crate) name: Option<&'ast str>,
}

#[derive(Debug, Default)]
pub(super) struct EnumData<'ast> {
    pub(super) mapped_name: Option<&'ast str>,
    /// @map on enum values.
    pub(super) mapped_values: HashMap<u32, &'ast str>,
}

fn visit_model<'ast>(model_id: ast::ModelId, ast_model: &'ast ast::Model, ctx: &mut Context<'ast>) {
    let model_data = ModelData {
        // This needs to be looked up first, because we want to skip some field
        // validations when the model is ignored.
        is_ignored: ast_model.attributes.iter().any(|attr| attr.name.name == "ignore"),
        ..Default::default()
    };

    for (field_id, ast_field) in ast_model.iter_fields() {
        match field_type(ast_field, ctx) {
            Ok(FieldType::Model(referenced_model)) => {
                let rf = RelationField::new(referenced_model);
                ctx.db.types.relation_fields.insert((model_id, field_id), rf);
            }
            Ok(FieldType::Scalar(scalar_field_type)) => {
                let field_data = ScalarField {
                    r#type: scalar_field_type,
                    is_ignored: false,
                    is_updated_at: false,
                    default: None,
                    mapped_name: None,
                };

                if matches!(scalar_field_type, ScalarFieldType::BuiltInScalar(t) if t.is_json())
                    && !ctx.db.active_connector().supports_json()
                {
                    ctx.push_error(DatamodelError::new_field_validation_error(
                        &format!("Field `{}` in model `{}` can't be of type Json. The current connector does not support the Json type.", &ast_field.name.name, &ast_model.name.name),
                        &ast_model.name.name,
                        &ast_field.name.name,
                        ast_field.span,
                    ));
                }

                if ast_field.arity.is_list() && !ctx.db.active_connector().supports_scalar_lists() {
                    ctx.push_error(DatamodelError::new_scalar_list_fields_are_not_supported(
                        &ast_model.name.name,
                        &ast_field.name.name,
                        ast_field.span,
                    ));
                }

                ctx.db.types.scalar_fields.insert((model_id, field_id), field_data);
            }
            Err(supported) => ctx.push_error(DatamodelError::new_type_not_found_error(
                supported,
                ast_field.field_type.span(),
            )),
        }
    }

    ctx.db.types.models.insert(model_id, model_data);
}

/// Detect self-referencing type aliases, possibly indirectly. We loop
/// through each type alias in the schema. If it references another type
/// alias — which may in turn reference another type alias —, we check that
/// it is not self-referencing. If a type alias ends up transitively
/// referencing itself, we create an error diagnostic.
fn detect_alias_cycles(ctx: &mut Context<'_>) {
    // The IDs of the type aliases we traversed to get to the current type alias.
    let mut path = Vec::new();
    // We accumulate the errors here because we want to sort them at the end.
    let mut errors: Vec<(ast::AliasId, DatamodelError)> = Vec::new();

    for (alias_id, ty) in &ctx.db.types.type_aliases {
        // Loop variable. This is the "tip" of the sequence of type aliases.
        let mut current = (*alias_id, ty);
        path.clear();

        // Follow the chain of type aliases referencing other type aliases.
        while let ScalarFieldType::Alias(next_alias_id) = current.1 {
            path.push(current.0);
            let next_alias = &ctx.db.ast[*next_alias_id];
            // Detect a cycle where next type is also the root. In that
            // case, we want to report an error.
            if path.len() > 1 && &path[0] == next_alias_id {
                errors.push((
                    *alias_id,
                    DatamodelError::new_validation_error(
                        &format!(
                            "Recursive type definitions are not allowed. Recursive path was: {} -> {}.",
                            path.iter().map(|id| &ctx.db.ast[*id].name.name).join(" -> "),
                            &next_alias.name.name,
                        ),
                        next_alias.field_type.span(),
                    ),
                ));
                break;
            }

            // We detect a cycle anywhere else in the chain of type aliases.
            // In that case, the error will be reported somewhere else, and
            // we can just move on from this alias.
            if path.contains(next_alias_id) {
                break;
            }

            match ctx.db.types.type_aliases.get(next_alias_id) {
                Some(next_alias_type) => {
                    current = (*next_alias_id, next_alias_type);
                }
                // A missing alias at this point means that there was an
                // error resolving the type of the next alias. We stop
                // validation here.
                None => break,
            }
        }
    }

    errors.sort_by_key(|(id, _err)| *id);
    for (_, error) in errors {
        ctx.push_error(error);
    }
}

fn visit_enum<'ast>(enum_id: ast::EnumId, enm: &'ast ast::Enum, ctx: &mut Context<'ast>) {
    let mut enum_data = EnumData::default();

    for (field_idx, field) in enm.values.iter().enumerate() {
        ctx.visit_attributes(&field.attributes, |attributes, ctx| {
            // @map
            attributes.visit_optional_single("map", ctx, |map_args, ctx| {
                if let Some(mapped_name) = attributes::visit_map(map_args, ctx) {
                    enum_data.mapped_values.insert(field_idx as u32, mapped_name);
                    ctx.mapped_enum_value_names
                        .insert((enum_id, mapped_name), field_idx as u32);
                }
            })
        });
    }

    ctx.visit_attributes(&enm.attributes, |attributes, ctx| {
        // @@map
        attributes.visit_optional_single("map", ctx, |map_args, ctx| {
            if let Some(mapped_name) = attributes::visit_map(map_args, ctx) {
                enum_data.mapped_name = Some(mapped_name);
                ctx.mapped_enum_names.insert(mapped_name, enum_id);
            }
        })
    });

    ctx.db.types.enums.insert(enum_id, enum_data);
}

fn visit_type_alias<'ast>(alias_id: ast::AliasId, alias: &'ast ast::Field, ctx: &mut Context<'ast>) {
    match field_type(alias, ctx) {
        Ok(FieldType::Scalar(scalar_field_type)) => {
            ctx.db.types.type_aliases.insert(alias_id, scalar_field_type);
        }
        Ok(FieldType::Model(_)) => ctx.push_error(DatamodelError::new_validation_error(
            "Only scalar types can be used for defining custom types.",
            alias.field_type.span(),
        )),
        Err(supported) => ctx.push_error(DatamodelError::new_type_not_found_error(
            supported,
            alias.field_type.span(),
        )),
    };
}

fn field_type<'ast>(field: &'ast ast::Field, ctx: &mut Context<'ast>) -> Result<FieldType, &'ast str> {
    let supported = match &field.field_type {
        ast::FieldType::Supported(ident) => &ident.name,
        ast::FieldType::Unsupported(_, _) => return Ok(FieldType::Scalar(ScalarFieldType::Unsupported)),
    };

    if let Ok(tpe) = dml::scalars::ScalarType::from_str(supported) {
        return Ok(FieldType::Scalar(ScalarFieldType::BuiltInScalar(tpe)));
    }

    match ctx
        .db
        .names
        .tops
        .get(supported.as_str())
        .map(|id| (*id, &ctx.db.ast[*id]))
    {
        Some((ast::TopId::Model(model_id), ast::Top::Model(_))) => Ok(FieldType::Model(model_id)),
        Some((ast::TopId::Enum(enum_id), ast::Top::Enum(_))) => Ok(FieldType::Scalar(ScalarFieldType::Enum(enum_id))),
        Some((ast::TopId::Alias(id), ast::Top::Type(_))) => Ok(FieldType::Scalar(ScalarFieldType::Alias(id))),
        Some((_, ast::Top::Generator(_))) | Some((_, ast::Top::Source(_))) => unreachable!(),
        None => Err(supported),
        _ => unreachable!(),
    }
}
