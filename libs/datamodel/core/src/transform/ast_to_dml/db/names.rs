pub(super) mod constraint_namespace;

use self::constraint_namespace::ConstraintNamespace;

use super::Context;
use crate::{
    ast::{self, Argument, TopId, WithAttributes, WithIdentifier},
    diagnostics::DatamodelError,
    reserved_model_names::{validate_enum_name, validate_model_name},
};
use datamodel_connector::ConstraintScope;
use dml::scalars::ScalarType;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    str::FromStr,
};

/// Resolved names for use in the validation process.
#[derive(Default)]
pub(super) struct Names<'ast> {
    /// Models, enums, composite types and type aliases
    pub(super) tops: HashMap<&'ast str, TopId>,
    /// Generators have their own namespace.
    pub(super) generators: HashMap<&'ast str, TopId>,
    /// Datasources have their own namespace.
    pub(super) datasources: HashMap<&'ast str, TopId>,
    pub(super) model_fields: BTreeMap<(ast::ModelId, &'ast str), ast::FieldId>,
    pub(super) composite_type_fields: HashMap<(ast::CompositeTypeId, &'ast str), ast::FieldId>,
    pub(super) constraint_namespace: ConstraintNamespace<'ast>,
}

/// `resolve_names()` is responsible for populating `ParserDatabase.names` and
/// validating that there are no name collisions in the following namespaces:
///
/// - Model, enum and type alias names
/// - Generators
/// - Datasources
/// - Model fields for each model
/// - Enum variants for each enum
pub(super) fn resolve_names(ctx: &mut Context<'_>) {
    let mut tmp_names: HashSet<&str> = HashSet::new(); // throwaway container for duplicate checking
    let mut names = Names::default();

    for (top_id, top) in ctx.db.ast.iter_tops() {
        assert_is_not_a_reserved_scalar_type(top.identifier(), ctx);

        let namespace = match (top_id, top) {
            (_, ast::Top::Enum(ast_enum)) => {
                tmp_names.clear();
                validate_identifier(&ast_enum.name, "Enum", ctx);
                validate_enum_name(ast_enum, &mut ctx.diagnostics);
                validate_attribute_identifiers(ast_enum, ctx);

                for value in &ast_enum.values {
                    validate_identifier(&value.name, "Enum Value", ctx);
                    validate_attribute_identifiers(value, ctx);

                    if !tmp_names.insert(&value.name.name) {
                        ctx.push_error(DatamodelError::new_duplicate_enum_value_error(
                            &ast_enum.name.name,
                            &value.name.name,
                            value.span,
                        ))
                    }
                }

                &mut names.tops
            }
            (ast::TopId::Model(model_id), ast::Top::Model(model)) => {
                validate_identifier(&model.name, "Model", ctx);
                validate_model_name(model, &mut ctx.diagnostics);
                validate_attribute_identifiers(model, ctx);

                for (field_id, field) in model.iter_fields() {
                    validate_identifier(&field.name, "Field", ctx);
                    validate_attribute_identifiers(field, ctx);

                    if names
                        .model_fields
                        .insert((model_id, &field.name.name), field_id)
                        .is_some()
                    {
                        ctx.push_error(DatamodelError::new_duplicate_field_error(
                            &model.name.name,
                            &field.name.name,
                            field.identifier().span,
                        ))
                    }
                }

                &mut names.tops
            }
            (ast::TopId::CompositeType(ctid), ast::Top::CompositeType(ct)) => {
                if !ctx.db.active_connector().supports_composite_types() {
                    ctx.push_error(DatamodelError::new_validation_error(
                        format!(
                            "Composite types are not supported on {}.",
                            ctx.db.active_connector().name()
                        ),
                        ct.span,
                    ));
                    continue;
                }

                validate_identifier(&ct.name, "Composite type", ctx);

                for (field_id, field) in ct.iter_fields() {
                    // Check that there is no duplicate field on the composite type
                    if names
                        .composite_type_fields
                        .insert((ctid, &field.name.name), field_id)
                        .is_some()
                    {
                        ctx.push_error(DatamodelError::new_composite_type_duplicate_field_error(
                            &ct.name.name,
                            &field.name.name,
                            field.identifier().span,
                        ))
                    }
                }

                &mut names.tops
            }
            (_, ast::Top::Source(datasource)) => {
                check_for_duplicate_properties(top, &datasource.properties, &mut tmp_names, ctx);
                &mut names.datasources
            }
            (_, ast::Top::Generator(generator)) => {
                check_for_duplicate_properties(top, &generator.properties, &mut tmp_names, ctx);
                &mut names.generators
            }
            (_, ast::Top::Type(_)) => &mut names.tops,
            _ => unreachable!(),
        };

        insert_name(top_id, top, namespace, ctx)
    }

    ctx.db.names = names;
}

/// Generate namespaces per database requirements, and add the names to it from the constraints
/// part of the namespace.
pub(super) fn infer_namespaces(ctx: &mut Context<'_>) {
    let mut namespaces = ConstraintNamespace::default();

    for scope in ctx.db.active_connector().constraint_violation_scopes() {
        match scope {
            ConstraintScope::GlobalKeyIndex => {
                namespaces.add_global_indexes(ctx, *scope);
            }
            ConstraintScope::GlobalForeignKey => {
                namespaces.add_global_relations(ctx, *scope);
            }
            ConstraintScope::GlobalPrimaryKeyKeyIndex => {
                namespaces.add_global_primary_keys(ctx, *scope);
                namespaces.add_global_indexes(ctx, *scope);
            }
            ConstraintScope::GlobalPrimaryKeyForeignKeyDefault => {
                namespaces.add_global_primary_keys(ctx, *scope);
                namespaces.add_global_relations(ctx, *scope);
                namespaces.add_global_default_constraints(ctx, *scope);
            }
            ConstraintScope::ModelKeyIndex => {
                namespaces.add_local_indexes(ctx, *scope);
            }
            ConstraintScope::ModelPrimaryKeyKeyIndex => {
                namespaces.add_local_primary_keys(ctx, *scope);
                namespaces.add_local_indexes(ctx, *scope);
            }
            ConstraintScope::ModelPrimaryKeyKeyIndexForeignKey => {
                namespaces.add_local_primary_keys(ctx, *scope);
                namespaces.add_local_indexes(ctx, *scope);
                namespaces.add_local_relations(ctx, *scope);
            }
        }
    }

    ctx.db.names.constraint_namespace = namespaces;
}

fn insert_name<'ast>(
    top_id: TopId,
    top: &'ast ast::Top,
    namespace: &mut HashMap<&'ast str, TopId>,
    ctx: &mut Context<'_>,
) {
    if let Some(existing) = namespace.insert(top.name(), top_id) {
        ctx.push_error(duplicate_top_error(&ctx.db.ast[existing], top));
    }
}

fn duplicate_top_error(existing: &ast::Top, duplicate: &ast::Top) -> DatamodelError {
    DatamodelError::new_duplicate_top_error(
        duplicate.name(),
        duplicate.get_type(),
        existing.get_type(),
        duplicate.identifier().span,
    )
}

fn assert_is_not_a_reserved_scalar_type(ident: &ast::Identifier, ctx: &mut Context<'_>) {
    if ScalarType::from_str(&ident.name).is_ok() {
        ctx.push_error(DatamodelError::new_reserved_scalar_type_error(&ident.name, ident.span));
    }
}

fn check_for_duplicate_properties<'a>(
    top: &ast::Top,
    props: &'a [Argument],
    tmp_names: &mut HashSet<&'a str>,
    ctx: &mut Context<'_>,
) {
    tmp_names.clear();
    for arg in props {
        if !tmp_names.insert(&arg.name.name) {
            ctx.push_error(DatamodelError::new_duplicate_config_key_error(
                &format!("{} \"{}\"", top.get_type(), top.name()),
                &arg.name.name,
                arg.identifier().span,
            ));
        }
    }
}

fn validate_attribute_identifiers(with_attrs: &dyn WithAttributes, ctx: &mut Context<'_>) {
    for attribute in with_attrs.attributes() {
        validate_identifier(&attribute.name, "Attribute", ctx);
    }
}

fn validate_identifier(ident: &ast::Identifier, schema_item: &str, ctx: &mut Context<'_>) {
    if ident.name.is_empty() {
        ctx.push_error(DatamodelError::new_validation_error(
            format!("The name of a {} must not be empty.", schema_item),
            ident.span,
        ))
    } else if ident.name.chars().next().unwrap().is_numeric() {
        ctx.push_error(DatamodelError::new_validation_error(
            format!("The name of a {} must not start with a number.", schema_item),
            ident.span,
        ))
    } else if ident.name.contains('-') {
        ctx.push_error(DatamodelError::new_validation_error(
            format!("The character `-` is not allowed in {} names.", schema_item),
            ident.span,
        ))
    }
}
