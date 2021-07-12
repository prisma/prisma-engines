use super::Context;
use crate::{
    ast::{self, Argument, FieldId, TopId, WithAttributes, WithIdentifier},
    diagnostics::DatamodelError,
};
use dml::scalars::ScalarType;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    str::FromStr,
};

/// Resolved names for use in the validation process.
///
/// `Names::new()` is also responsible for validating that there are no name
/// collisions in the following namespaces:
///
/// - Model, enum and type alias names
/// - Generators
/// - Datasources
/// - Model fields for each model
/// - Enum variants for each enum
#[derive(Default)]
pub(super) struct Names<'ast> {
    /// Models, enums and type aliases
    pub(super) tops: HashMap<&'ast str, TopId>,
    /// Generators have their own namespace.
    pub(super) generators: HashMap<&'ast str, TopId>,
    /// Datasources have their own namespace.
    pub(super) datasources: HashMap<&'ast str, TopId>,
    pub(super) model_fields: BTreeMap<(ast::ModelId, &'ast str), FieldId>,
}

pub(super) fn resolve_names(ctx: &mut Context<'_>) {
    let mut tmp_names: HashSet<&str> = HashSet::new(); // throwaway container for duplicate checking
    let mut names = Names::default();

    for (top_id, top) in ctx.db.ast.iter_tops() {
        assert_is_not_a_reserved_scalar_type(top, ctx);

        let namespace = match (top_id, top) {
            (_, ast::Top::Enum(ast_enum)) => {
                tmp_names.clear();
                ast_enum.name.validate("Enum", &mut ctx.diagnostics);
                ast_enum.validate_attributes(&mut ctx.diagnostics);

                for value in &ast_enum.values {
                    value.name.validate("Enum Value", &mut ctx.diagnostics);
                    value.validate_attributes(&mut ctx.diagnostics);

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
                model.name.validate("Model", &mut ctx.diagnostics);
                model.validate_attributes(&mut ctx.diagnostics);

                for (field_id, field) in model.iter_fields() {
                    field.name.validate("Field", &mut ctx.diagnostics);
                    field.validate_attributes(&mut ctx.diagnostics);

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

fn assert_is_not_a_reserved_scalar_type(top: &dyn WithIdentifier, ctx: &mut Context<'_>) {
    let ident = top.identifier();
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
