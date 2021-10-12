use super::context::Context;
use crate::{ast, diagnostics::DatamodelError};
use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;
use std::{
    collections::{BTreeMap, HashMap},
    str::FromStr,
};

pub(super) fn resolve_types(ctx: &mut Context<'_>) {
    for (top_id, top) in ctx.db.ast.iter_tops() {
        match (top_id, top) {
            (ast::TopId::Alias(alias_id), ast::Top::Type(type_alias)) => visit_type_alias(alias_id, type_alias, ctx),
            (ast::TopId::Model(model_id), ast::Top::Model(model)) => visit_model(model_id, model, ctx),
            (ast::TopId::Enum(_), ast::Top::Enum(enm)) => visit_enum(enm, ctx),
            (_, ast::Top::Source(_)) | (_, ast::Top::Generator(_)) => (),
            _ => unreachable!(),
        }
    }

    detect_alias_cycles(ctx);
}

#[derive(Debug, Default)]
pub(crate) struct Types<'ast> {
    pub(super) type_aliases: HashMap<ast::AliasId, ScalarFieldType>,
    pub(super) scalar_fields: BTreeMap<(ast::ModelId, ast::FieldId), ScalarField<'ast>>,
    /// This contains only the relation fields actually present in the schema
    /// source text.
    pub(crate) relation_fields: BTreeMap<(ast::ModelId, ast::FieldId), RelationField<'ast>>,
    pub(super) enum_attributes: HashMap<ast::EnumId, EnumAttributes<'ast>>,
    pub(super) model_attributes: HashMap<ast::ModelId, ModelAttributes<'ast>>,
}

impl<'ast> Types<'ast> {
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

#[derive(Debug, Clone, Copy, PartialEq)]
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
    pub(crate) mapped_name: Option<&'ast str>,
    // Native type name and arguments
    pub(crate) native_type: Option<(&'ast str, Vec<String>)>,
}

impl ScalarField<'_> {
    pub(crate) fn is_autoincrement(&self) -> bool {
        matches!(&self.default.as_ref().map(|d| d.kind()), Some(crate::dml::DefaultKind::Expression(expr)) if expr.is_autoincrement())
    }
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
    /// The fk_name _explicitly present_ in the AST through the map argument.
    pub(crate) fk_name: Option<&'ast str>,
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
            fk_name: None,
        }
    }
}

/// Information gathered from validating attributes on a model.
#[derive(Default, Debug)]
pub(crate) struct ModelAttributes<'ast> {
    /// @(@)id
    pub(super) primary_key: Option<IdAttribute<'ast>>,
    /// @@ignore
    pub(crate) is_ignored: bool,
    /// @@index and @(@)unique.
    pub(super) indexes: Vec<(&'ast ast::Attribute, IndexAttribute<'ast>)>,
    /// @@map
    pub(crate) mapped_name: Option<&'ast str>,
}

impl ModelAttributes<'_> {
    /// Whether the field is the whole primary key. Will match `@id` and `@@id([fieldName])`.
    pub(super) fn field_is_single_pk(&self, field: ast::FieldId) -> bool {
        self.primary_key.as_ref().filter(|pk| pk.fields == [field]).is_some()
    }

    /// Whether MySQL would consider the field indexed for autoincrement purposes.
    pub(super) fn field_is_indexed_for_autoincrement(&self, field_id: ast::FieldId) -> bool {
        self.indexes.iter().any(|(_, idx)| idx.fields.get(0) == Some(&field_id))
            || self
                .primary_key
                .as_ref()
                .filter(|pk| pk.fields.get(0) == Some(&field_id))
                .is_some()
    }
}

#[derive(Debug, Default)]
pub(crate) struct IndexAttribute<'ast> {
    pub(crate) is_unique: bool,
    pub(crate) fields: Vec<ast::FieldId>,
    pub(crate) source_field: Option<ast::FieldId>,
    pub(crate) name: Option<&'ast str>,
    pub(crate) db_name: Option<&'ast str>,
}

#[derive(Debug, Default)]
pub(super) struct IdAttribute<'ast> {
    pub(super) fields: Vec<ast::FieldId>,
    pub(super) source_field: Option<ast::FieldId>,
    pub(super) name: Option<&'ast str>,
    pub(super) db_name: Option<&'ast str>,
}

#[derive(Debug, Default)]
pub(super) struct EnumAttributes<'ast> {
    pub(super) mapped_name: Option<&'ast str>,
    /// @map on enum values.
    pub(super) mapped_values: HashMap<u32, &'ast str>,
}

fn visit_model<'ast>(model_id: ast::ModelId, ast_model: &'ast ast::Model, ctx: &mut Context<'ast>) {
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
                    native_type: None,
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

                if matches!(scalar_field_type, ScalarFieldType::Unsupported) {
                    validate_unsupported_field_type(ast_field, ast_field.field_type.as_unsupported().unwrap().0, ctx);
                }

                ctx.db.types.scalar_fields.insert((model_id, field_id), field_data);
            }
            Err(supported) => ctx.push_error(DatamodelError::new_type_not_found_error(
                supported,
                ast_field.field_type.span(),
            )),
        }
    }
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

fn visit_enum<'ast>(enm: &'ast ast::Enum, ctx: &mut Context<'ast>) {
    if !ctx.db.active_connector().supports_enums() {
        ctx.push_error(DatamodelError::new_validation_error(
            &format!(
                "You defined the enum `{}`. But the current connector does not support enums.",
                &enm.name.name
            ),
            enm.span,
        ));
    }

    if enm.values.is_empty() {
        ctx.push_error(DatamodelError::new_validation_error(
            "An enum must have at least one value.",
            enm.span,
        ))
    }
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

fn validate_unsupported_field_type(ast_field: &ast::Field, unsupported_lit: &str, ctx: &mut Context<'_>) {
    static TYPE_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#"(?x)
    ^                           # beginning of the string
    (?P<prefix>[^(]+)           # a required prefix that is any character until the first opening brace
    (?:\((?P<params>.*?)\))?    # (optional) an opening parenthesis, a closing parenthesis and captured params in-between
    (?P<suffix>.+)?             # (optional) captured suffix after the params until the end of the string
    $                           # end of the string
    "#).unwrap()
    });

    if let Some(source) = ctx.db.datasource() {
        let connector = &source.active_connector;

        if let Some(captures) = TYPE_REGEX.captures(unsupported_lit) {
            let prefix = captures.name("prefix").unwrap().as_str().trim();

            let params = captures.name("params");
            let args = match params {
                None => vec![],
                Some(params) => params.as_str().split(',').map(|s| s.trim().to_string()).collect(),
            };

            if let Ok(native_type) = connector.parse_native_type(prefix, args) {
                let prisma_type = connector.scalar_type_for_native_type(native_type.serialized_native_type.clone());

                let msg = format!(
                        "The type `Unsupported(\"{}\")` you specified in the type definition for the field `{}` is supported as a native type by Prisma. Please use the native type notation `{} @{}.{}` for full support.",
                        unsupported_lit, ast_field.name.name, prisma_type.to_string(), &source.name, native_type.render()
                    );

                ctx.push_error(DatamodelError::new_validation_error(&msg, ast_field.span));
            }
        }
    }
}
