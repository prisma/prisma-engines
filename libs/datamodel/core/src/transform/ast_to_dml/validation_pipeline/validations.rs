mod autoincrement;
mod composite_types;
mod constraint_namespace;
mod database_name;
mod fields;
mod indexes;
mod models;
mod names;
mod relation_fields;
mod relations;

use super::context::Context;
use crate::{ast, transform::ast_to_dml::db::walkers::RefinedRelationWalker};
use diagnostics::DatamodelError;
use names::Names;

pub(super) fn validate(ctx: &mut Context<'_>, relation_transformation_enabled: bool) {
    let db = ctx.db;
    let connector = ctx.connector;

    let names = Names::new(db, connector);

    for composite_type in db.walk_composite_types() {
        composite_types::composite_types_support(composite_type, ctx);
    }

    for model in db.walk_models() {
        models::has_a_strict_unique_criteria(model, ctx);
        models::has_a_unique_primary_key_name(model, &names, ctx);
        models::has_a_unique_custom_primary_key_name_per_model(model, &names, ctx);
        models::uses_sort_or_length_on_primary_without_preview_flag(model, ctx);
        models::id_has_fields(model, ctx);
        models::primary_key_connector_specific(model, ctx);
        models::primary_key_length_prefix_supported(model, ctx);
        models::primary_key_sort_order_supported(model, ctx);
        models::only_one_fulltext_attribute_allowed(model, ctx);
        models::connector_specific(model, ctx);
        autoincrement::validate_auto_increment(model, ctx);

        if let Some(pk) = model.primary_key() {
            for field_attribute in pk.scalar_field_attributes() {
                let span = pk.ast_attribute().span;
                let attribute = ("id", span);
                fields::validate_length_used_with_correct_types(field_attribute, attribute, ctx);
            }
        }

        for field in model.scalar_fields() {
            fields::validate_scalar_field_connector_specific(field, ctx);
            fields::validate_client_name(field.into(), &names, ctx);
            fields::has_a_unique_default_constraint_name(field, &names, ctx);
            fields::validate_native_type_arguments(field, ctx);
            fields::validate_default(field, ctx);
            fields::validate_unsupported_field_type(field, ctx)
        }

        for field in model.relation_fields() {
            // We don't want to spam, so with ambiguous relations we should exit
            // immediately if any errors.
            if let Err(error) = relation_fields::ambiguity(field, &names) {
                ctx.push_error(error);
                return;
            }

            fields::validate_client_name(field.into(), &names, ctx);

            relation_fields::ignored_related_model(field, ctx);
            relation_fields::referential_actions(field, ctx);
            relation_fields::map(field, ctx);
        }

        for index in model.indexes() {
            indexes::has_fields(index, ctx);
            indexes::has_a_unique_constraint_name(index, &names, ctx);
            indexes::unique_index_has_a_unique_custom_name_per_model(index, &names, ctx);
            indexes::uses_length_or_sort_without_preview_flag(index, ctx);
            indexes::field_length_prefix_supported(index, ctx);
            indexes::index_algorithm_preview_feature(index, ctx);
            indexes::index_algorithm_is_supported(index, ctx);
            indexes::hash_index_must_not_use_sort_param(index, ctx);
            indexes::fulltext_index_preview_feature_enabled(index, ctx);
            indexes::fulltext_index_supported(index, ctx);
            indexes::fulltext_columns_should_not_define_length(index, ctx);
            indexes::fulltext_column_sort_is_supported(index, ctx);
            indexes::fulltext_text_columns_should_be_bundled_together(index, ctx);
            indexes::has_valid_mapped_name(index, ctx);

            for field_attribute in index.scalar_field_attributes() {
                let span = index
                    .ast_attribute()
                    .map(|attr| attr.span)
                    .unwrap_or_else(ast::Span::empty);

                let attribute = (index.attribute_name(), span);
                fields::validate_length_used_with_correct_types(field_attribute, attribute, ctx);
            }
        }
    }

    if !connector.supports_enums() {
        for r#enum in db.ast().iter_tops().filter_map(|(_, top)| top.as_enum()) {
            ctx.push_error(DatamodelError::new_validation_error(
                format!(
                    "You defined the enum `{}`. But the current connector does not support enums.",
                    &r#enum.name.name
                ),
                r#enum.span,
            ));
        }
    }

    for relation in db.walk_relations() {
        match relation.refine() {
            // 1:1, 1:n
            RefinedRelationWalker::Inline(relation) => {
                if let Some(relation) = relation.as_complete() {
                    relations::cycles(relation, ctx);
                    relations::multiple_cascading_paths(relation, ctx);
                }

                relations::references_unique_fields(relation, ctx);
                relations::same_length_in_referencing_and_referenced(relation, ctx);
                relations::referencing_fields_in_correct_order(relation, ctx);
                relations::field_arity(relation, ctx);
                relations::referencing_scalar_field_types(relation, ctx);
                relations::has_a_unique_constraint_name(&names, relation, ctx);

                // Only run these when you are not formatting the data model. These validations
                // test against broken relations that we could fix with a code action. The flag is
                // set when prisma-fmt calls this code.
                if !relation_transformation_enabled {
                    if relation.is_one_to_one() {
                        relations::one_to_one::both_sides_are_defined(relation, ctx);
                        relations::one_to_one::fields_and_references_are_defined(relation, ctx);
                        relations::one_to_one::fields_and_references_defined_on_one_side_only(relation, ctx);
                        relations::one_to_one::referential_actions(relation, ctx);

                        // Run these validations last to prevent validation spam.
                        relations::one_to_one::fields_references_mixups(relation, ctx);
                        relations::one_to_one::back_relation_arity_is_optional(relation, ctx);
                        relations::one_to_one::fields_and_references_on_wrong_side(relation, ctx);
                    } else {
                        relations::one_to_many::both_sides_are_defined(relation, ctx);
                        relations::one_to_many::fields_and_references_are_defined(relation, ctx);
                        relations::one_to_many::referential_actions(relation, ctx);
                    }
                }
            }
            RefinedRelationWalker::ImplicitManyToMany(relation) => {
                relations::many_to_many::validate_singular_id(relation, ctx);
                relations::many_to_many::validate_no_referential_actions(relation, ctx);
            }
        }
    }
}
