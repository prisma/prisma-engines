mod autoincrement;
mod composite_types;
mod fields;
mod indexes;
mod models;
mod names;
mod relation_fields;
mod relations;

use self::names::Names;
use crate::{
    ast,
    diagnostics::Diagnostics,
    transform::ast_to_dml::db::{walkers::RefinedRelationWalker, ParserDatabase},
};

pub(super) fn validate(db: &ParserDatabase<'_>, diagnostics: &mut Diagnostics, relation_transformation_enabled: bool) {
    let connector = db.active_connector();
    let names = Names::new(db);

    for composite_type in db.walk_composite_types() {
        composite_types::composite_types_support(composite_type, connector, diagnostics);
    }

    for model in db.walk_models() {
        models::has_a_strict_unique_criteria(model, diagnostics);
        models::has_a_unique_primary_key_name(db, model, diagnostics);
        models::uses_sort_or_length_on_primary_without_preview_flag(db, model, diagnostics);
        models::primary_key_length_prefix_supported(db, model, diagnostics);
        models::primary_key_sort_order_supported(db, model, diagnostics);
        models::only_one_fulltext_attribute_allowed(db, model, diagnostics);
        autoincrement::validate_auto_increment(model, connector, diagnostics);

        if let Some(pk) = model.primary_key() {
            for field_attribute in pk.scalar_field_attributes() {
                let span = pk.ast_attribute().span;
                let attribute = ("id", span);
                fields::validate_length_used_with_correct_types(db, field_attribute, attribute, diagnostics);
            }
        }

        for field in model.scalar_fields() {
            fields::validate_client_name(field.into(), &names, diagnostics);
            fields::has_a_unique_default_constraint_name(db, field, diagnostics);
            fields::validate_native_type_arguments(field, diagnostics);
            fields::validate_default(field, connector, diagnostics);
        }

        for field in model.relation_fields() {
            // We don't want to spam, so with ambiguous relations we should exit
            // immediately if any errors.
            if let Err(error) = relation_fields::ambiguity(field, &names) {
                diagnostics.push_error(error);
                return;
            }

            fields::validate_client_name(field.into(), &names, diagnostics);

            relation_fields::ignored_related_model(field, diagnostics);
            relation_fields::referential_actions(field, db, diagnostics);
            relation_fields::map(field, diagnostics);
        }

        for index in model.indexes() {
            indexes::has_a_unique_constraint_name(db, index, diagnostics);
            indexes::uses_length_or_sort_without_preview_flag(db, index, diagnostics);
            indexes::field_length_prefix_supported(db, index, diagnostics);
            indexes::index_algorithm_preview_feature(db, index, diagnostics);
            indexes::index_algorithm_is_supported(db, index, diagnostics);
            indexes::hash_index_must_not_use_sort_param(db, index, diagnostics);
            indexes::fulltext_index_preview_feature_enabled(db, index, diagnostics);
            indexes::fulltext_index_supported(db, index, diagnostics);
            indexes::fulltext_columns_should_not_define_length(db, index, diagnostics);
            indexes::fulltext_column_sort_is_supported(db, index, diagnostics);
            indexes::fulltext_text_columns_should_be_bundled_together(db, index, diagnostics);

            for field_attribute in index.scalar_field_attributes() {
                let span = index
                    .ast_attribute()
                    .map(|attr| attr.span)
                    .unwrap_or_else(ast::Span::empty);

                let attribute = (index.attribute_name(), span);
                fields::validate_length_used_with_correct_types(db, field_attribute, attribute, diagnostics);
            }
        }
    }

    for relation in db.walk_relations() {
        match relation.refine() {
            // 1:1, 1:n
            RefinedRelationWalker::Inline(relation) => {
                if let Some(relation) = relation.as_complete() {
                    relations::field_arity(relation, diagnostics);
                    relations::same_length_in_referencing_and_referenced(relation, diagnostics);
                    relations::cycles(relation, db, diagnostics);
                    relations::multiple_cascading_paths(relation, db, diagnostics);
                    relations::has_a_unique_constraint_name(db, relation, diagnostics);
                    relations::references_unique_fields(relation, connector, diagnostics);
                    relations::referencing_fields_in_correct_order(relation, connector, diagnostics);
                }

                relations::referencing_scalar_field_types(relation, diagnostics);

                // Only run these when you are not formatting the data model. These validations
                // test against broken relations that we could fix with a code action. The flag is
                // set when prisma-fmt calls this code.
                if !relation_transformation_enabled {
                    if relation.is_one_to_one() {
                        relations::one_to_one::both_sides_are_defined(relation, diagnostics);
                        relations::one_to_one::fields_and_references_are_defined(relation, diagnostics);
                        relations::one_to_one::fields_and_references_defined_on_one_side_only(relation, diagnostics);
                        relations::one_to_one::referential_actions(relation, diagnostics);

                        // Run these validations last to prevent validation spam.
                        relations::one_to_one::fields_references_mixups(relation, diagnostics);
                        relations::one_to_one::back_relation_arity_is_optional(relation, diagnostics);
                    } else {
                        relations::one_to_many::both_sides_are_defined(relation, diagnostics);
                        relations::one_to_many::fields_and_references_are_defined(relation, diagnostics);
                        relations::one_to_many::referential_actions(relation, diagnostics);
                    }
                }
            }
            RefinedRelationWalker::ImplicitManyToMany(relation) => {
                relations::many_to_many::validate_singular_id(relation, diagnostics);
            }
        }
    }
}
