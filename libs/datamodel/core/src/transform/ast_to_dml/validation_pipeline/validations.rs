mod fields;
mod models;
mod names;
mod relation_fields;
mod relations;

use self::names::Names;
use crate::{
    diagnostics::Diagnostics,
    transform::ast_to_dml::db::{walkers::RefinedRelationWalker, ParserDatabase},
};

pub(super) fn validate(db: &ParserDatabase<'_>, diagnostics: &mut Diagnostics) {
    let names = Names::new(db);
    let connector = db.active_connector();

    let referential_integrity = db.datasource().map(|ds| ds.referential_integrity()).unwrap_or_default();

    for model in db.walk_models() {
        for field in model.scalar_fields() {
            fields::validate_client_name(field.into(), &names, diagnostics);
        }

        models::has_a_strict_unique_criteria(model, diagnostics);

        for field in model.relation_fields() {
            // We don't want to spam, so with ambiguous relations we should exit
            // immediately if any errors.

            if let Err(error) = relation_fields::ambiguity(field, &names) {
                diagnostics.push_error(error);
                return;
            }

            fields::validate_client_name(field.into(), &names, diagnostics);

            relation_fields::ignored_related_model(field, diagnostics);
            relation_fields::referential_actions(field, connector, diagnostics);
            relation_fields::on_update_without_foreign_keys(field, referential_integrity, diagnostics);
        }

        for relation in model.explicit_complete_relations_fwd() {
            relations::field_arity(relation, diagnostics);
            relations::same_length_in_referencing_and_referenced(relation, diagnostics);
            relations::cycles(relation, connector, diagnostics);
            relations::multiple_cascading_paths(relation, connector, diagnostics);

            // These needs to run last to prevent error spam.
            relations::references_unique_fields(relation, connector, diagnostics);
            relations::referencing_fields_in_correct_order(relation, connector, diagnostics);
        }
    }

    for relation in db
        .walk_relations()
        .filter_map(|relation| relation.refine().as_many_to_many())
    {
        relations::many_to_many::validate_singular_id(relation, diagnostics);
    }
}

pub(super) fn validate_strict(db: &ParserDatabase<'_>, diagnostics: &mut Diagnostics) {
    for relation in db.walk_relations() {
        match relation.refine() {
            RefinedRelationWalker::Inline(relation) if relation.is_one_to_one() => {
                relations::one_to_one::validate_strict(relation, diagnostics);
            }
            RefinedRelationWalker::Inline(relation) => {
                relations::one_to_many::validate_strict(relation, diagnostics);
            }
            RefinedRelationWalker::ImplicitManyToMany(_) => {}
        }
    }
}
