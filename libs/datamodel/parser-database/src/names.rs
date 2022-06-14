mod reserved_model_names;

pub use reserved_model_names::is_reserved_type_name;

use crate::{
    ast::{self, ConfigBlockProperty, TopId, WithAttributes, WithIdentifier},
    types::ScalarType,
    Context, DatamodelError, StringId,
};
use reserved_model_names::{validate_enum_name, validate_model_name};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Resolved names for use in the validation process.
#[derive(Default)]
pub(super) struct Names {
    /// Models, enums, composite types and type aliases
    pub(super) tops: HashMap<StringId, TopId>,
    /// Generators have their own namespace.
    pub(super) generators: HashMap<StringId, TopId>,
    /// Datasources have their own namespace.
    pub(super) datasources: HashMap<StringId, TopId>,
    pub(super) model_fields: BTreeMap<(ast::ModelId, StringId), ast::FieldId>,
    pub(super) composite_type_fields: HashMap<(ast::CompositeTypeId, StringId), ast::FieldId>,
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

    for (top_id, top) in ctx.ast.iter_tops() {
        assert_is_not_a_reserved_scalar_type(top.identifier(), ctx);

        let namespace = match (top_id, top) {
            (_, ast::Top::Enum(ast_enum)) => {
                tmp_names.clear();
                validate_identifier(&ast_enum.name, "Enum", ctx);
                validate_enum_name(ast_enum, ctx.diagnostics);
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
                validate_model_name(model, ctx.diagnostics);
                validate_attribute_identifiers(model, ctx);

                for (field_id, field) in model.iter_fields() {
                    validate_identifier(&field.name, "Field", ctx);
                    validate_attribute_identifiers(field, ctx);
                    let field_name_id = ctx.interner.intern(field.name());

                    if names.model_fields.insert((model_id, field_name_id), field_id).is_some() {
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
                validate_identifier(&ct.name, "Composite type", ctx);

                for (field_id, field) in ct.iter_fields() {
                    let field_name_id = ctx.interner.intern(field.name());
                    // Check that there is no duplicate field on the composite type
                    if names
                        .composite_type_fields
                        .insert((ctid, field_name_id), field_id)
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
            _ => unreachable!(),
        };

        insert_name(top_id, top, namespace, ctx)
    }

    let _ = std::mem::replace(ctx.names, names);
}

fn insert_name(top_id: TopId, top: &ast::Top, namespace: &mut HashMap<StringId, TopId>, ctx: &mut Context<'_>) {
    let name = ctx.interner.intern(top.name());
    if let Some(existing) = namespace.insert(name, top_id) {
        ctx.push_error(duplicate_top_error(&ctx.ast[existing], top));
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
    if ScalarType::try_from_str(&ident.name).is_some() {
        ctx.push_error(DatamodelError::new_reserved_scalar_type_error(&ident.name, ident.span));
    }
}

fn check_for_duplicate_properties<'a>(
    top: &ast::Top,
    props: &'a [ConfigBlockProperty],
    tmp_names: &mut HashSet<&'a str>,
    ctx: &mut Context<'_>,
) {
    tmp_names.clear();
    for arg in props {
        if !tmp_names.insert(&arg.name.name) {
            ctx.push_error(DatamodelError::new_duplicate_config_key_error(
                &format!("{} \"{}\"", top.get_type(), top.name()),
                &arg.name.name,
                arg.name.span,
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
