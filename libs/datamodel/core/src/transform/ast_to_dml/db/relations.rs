use super::{context::Context, types::RelationField};
use crate::ast;
use std::collections::BTreeSet;

#[derive(Debug, Default)]
pub(super) struct Relations<'ast> {
    /// (model_a_id, model_b_id, relation)
    many_to_many: BTreeSet<(ast::ModelId, ast::ModelId, ManyToManyRelation<'ast>)>,
    ///
    one_to_one: Vec<OneToOneRelation<'ast>>,
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug)]
struct ManyToManyRelation<'ast> {
    /// Relation field on model A.
    field_a: ast::FieldId,
    /// Relation field on model B.
    field_b: ast::FieldId,
    /// The `name` argument in `@relation`.
    relation_name: Option<&'ast str>,
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug)]
struct OneToOneRelation<'ast> {
    /// Relation field on the referencing model.
    referencing_field_id: ast::FieldId,
    /// Relation field on the referenced model.
    referenced_field_id: ast::FieldId,
    relation_name: Option<&'ast str>,
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug)]
struct ConcreteManyToOneRelation<'ast> {
    /// Relation field on the referencing model.
    referencing_field_id: ast::FieldId,
    /// Relation field on the referenced model.
    referenced_field_id: Option<ast::FieldId>,
    relation_name: Option<&'ast str>,
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug)]
struct VirtualManyToOneRelation<'ast> {
    /// Relation field on the referencing model.
    referencing_field_id: ast::FieldId,
    /// Relation field on the referenced model.
    referenced_field_id: Option<ast::FieldId>,
    relation_name: Option<&'ast str>,
}

pub(super) fn infer_relations(ctx: &mut Context<'_>) {
    let mut relations = Relations::default();

    for rf in ctx.db.types.relation_fields.iter() {
        let evidence = relation_evidence(rf, ctx);
        ingest_relation(evidence, &mut relations, ctx);
    }
}

// Implementation detail for this module. Should stay private.
struct RelationEvidence<'ast, 'db> {
    ast_model: &'ast ast::Model,
    model_id: ast::ModelId,
    ast_field: &'ast ast::Field,
    field_id: ast::FieldId,
    is_self_relation: bool,
    relation_field: &'db RelationField<'ast>,
    opposite_model: &'ast ast::Model,
    opposite_relation_field: Option<(ast::FieldId, &'ast ast::Field, &'db RelationField<'ast>)>,
}

fn relation_evidence<'ast, 'db>(
    ((model_id, field_id), relation_field): (&(ast::ModelId, ast::FieldId), &'db RelationField<'ast>),
    ctx: &'db mut Context<'ast>,
) -> RelationEvidence<'ast, 'db> {
    let ast_model = &ctx.db.ast[*model_id];
    let ast_field = &ast_model[*field_id];
    let opposite_model = &ctx.db.ast[relation_field.referenced_model];
    let is_self_relation = *model_id == relation_field.referenced_model;
    let opposite_relation_field: Option<(ast::FieldId, &ast::Field, &RelationField<'_>)> = ctx
        .db
        .iter_model_relation_fields(relation_field.referenced_model)
        // Only considers relations between the same models
        .filter(|(_, opposite_relation_field)| opposite_relation_field.referenced_model == *model_id)
        .filter(|(opposite_field_id, _)| !is_self_relation || opposite_field_id != field_id)
        // Filter out the field itself, in case of self-relations
        .filter(|(opposite_field_id, _)| !is_self_relation || opposite_field_id != field_id)
        .find(|(_, opp)| opp.name == relation_field.name)
        .map(|(opp_field_id, opp)| (opp_field_id, &opposite_model[opp_field_id], opp));

    RelationEvidence {
        ast_model,
        model_id: *model_id,
        ast_field,
        field_id: *field_id,
        relation_field,
        opposite_model,
        is_self_relation,
        opposite_relation_field,
    }
}

fn ingest_relation<'ast, 'db>(
    evidence: RelationEvidence<'ast, 'db>,
    relations: &mut Relations<'ast>,
    ctx: &Context<'ast>,
) {
    match (evidence.ast_field.arity, evidence.opposite_relation_field) {
        (ast::FieldArity::List, Some((opp_field_id, opp_field, _))) if opp_field.arity.is_list() => {
            // This is an implicit many-to-many relation.

            // We will meet the relation twice when we walk over all relation
            // fields, so we only instantiate it when the relation field is that
            // of model A, and the opposite is model B.
            if evidence.ast_model.name.name > evidence.opposite_model.name.name {
                return;
            }

            // For self-relations, the ordering logic is different: model A
            // and model B are the same. The _first field in source text
            // order_ is field A, the second field is field B.
            if evidence.is_self_relation && evidence.ast_field.name.name > opp_field.name.name {
                return;
            }

            let relation = ManyToManyRelation {
                field_a: evidence.field_id,
                field_b: opp_field_id,
                relation_name: evidence.relation_field.name,
            };

            relations
                .many_to_many
                .insert((evidence.model_id, evidence.relation_field.referenced_model, relation));
        }
        (ast::FieldArity::List, Some((opp_field_id, opp_field, opp_relation_field))) => {
            // This is a 1:m relation defined on both sides. We skip the virtual side.
        }
        (ast::FieldArity::List, None) => {
            // This is a 1:m relation defined on the virtual side only.
            todo!()
        }
        (ast::FieldArity::Optional | ast::FieldArity::Required, Some((_, opp_field, _)))
            if opp_field.arity.is_required() || opp_field.arity.is_optional() =>
        {
            // This is a 1:1 relation.
            todo!()
        }
        (ast::FieldArity::Optional | ast::FieldArity::Required, opp_field) => {
            // This is a 1:m relation.
            todo!()
        }
    };
}
