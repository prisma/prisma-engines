use super::{context::Context, types::RelationField};
use crate::ast;
use std::collections::BTreeSet;

#[derive(Debug, Default)]
pub(super) struct Relations<'ast> {
    /// (model_a_id, model_b_id, relation)
    many_to_many: BTreeSet<(ast::ModelId, ast::ModelId, ManyToManyRelation<'ast>)>,
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

pub(super) fn infer_relations(ctx: &mut Context<'_>) {
    for ((model_id, field_id), relation_field) in ctx.db.types.relation_fields.iter() {
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

        match (ast_field.arity, opposite_relation_field) {
            (ast::FieldArity::List, Some((opp_field_id, opp_field, _))) if opp_field.arity.is_list() => {
                // This is an implicit many-to-many relation.
                //
                // We will meet it twice when we walk over all relation fields,
                // so we only instantiate it when the relation field is that of
                // model A, and the opposite is model B.
                if ast_model.name.name > opposite_model.name.name {
                    continue;
                }

                // For self-relations, the ordering logic is different: model A
                // and model B are the same. The _first field in source text
                // order_ is field A, the second field is field B.
                if is_self_relation && *field_id > opp_field_id {
                    continue;
                }

                let relation = ManyToManyRelation {
                    field_a: *field_id,
                    field_b: opp_field_id,
                    relation_name: relation_field.name,
                };

                ctx.db
                    .relations
                    .many_to_many
                    .insert((*model_id, relation_field.referenced_model, relation));
            }
            _ => (),
        };
    }
}
