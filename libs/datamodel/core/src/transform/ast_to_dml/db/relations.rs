mod ingest;
mod validate;

use super::context::Context;
pub(crate) use ingest::*;

/// Detect relation types and construct relation objects to the database.
pub(super) fn infer_relations(ctx: &mut Context<'_>) {
    let mut relations = Relations::default();

    for rf in ctx.db.types.relation_fields.iter() {
        let evidence = relation_evidence(rf, ctx);
        ingest_relation(evidence, &mut relations, ctx);
    }

    ctx.db.relations = relations;
}

/// Validation of relation objects, should be run after inferring.
pub(super) fn validate_relations(ctx: &mut Context<'_>) {
    let mut errors = Vec::new();
    let connector = ctx.db.active_connector();

    // Complete 1:n and 1:1 relations.
    for relation in ctx.db.walk_explicit_relations() {
        validate::field_arity(relation, &mut errors);
        validate::same_length_in_referencing_and_referenced(relation, &mut errors);
        validate::cycles(relation, connector, &mut errors);
        validate::multiple_cascading_paths(relation, connector, &mut errors);

        // These needs to run last to prevent error spam.
        validate::references_unique_fields(relation, connector, &mut errors);
        validate::referencing_fields_in_correct_order(relation, connector, &mut errors);
    }

    for error in errors.into_iter() {
        ctx.push_error(error);
    }
}
