use super::{context::Context, types::RelationField};
use crate::ast;
use std::collections::BTreeSet;

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug, Clone)]
enum OneToManyRelationFields {
    Forward(ast::FieldId),
    Back(ast::FieldId),
    Both(ast::FieldId, ast::FieldId),
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug, Clone)]
enum RelationType {
    ImplicitManyToMany {
        field_a: ast::FieldId,
        field_b: ast::FieldId,
    },
    OneToOne {
        field_a: ast::FieldId,
        field_b: ast::FieldId,
    },
    OneToMany(OneToManyRelationFields),
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug, Clone)]
pub(super) struct Relation<'ast> {
    /// The `name` argument in `@relation`.
    relation_name: Option<&'ast str>,
    tpe: RelationType,
}

#[derive(Debug, Default)]
pub(super) struct Relations<'ast> {
    // A relation is always between two models. One model is assigned the role
    // of "model A", and the other is "model B". The meaning of "model A" and
    // "model B" depends on the type of relation.
    //
    // - In implicit many-to-many relations, model A and model B are ordered
    //   lexicographically, by model name, and failing that by relation field
    //   name. This order must be stable in order for the columns in the
    //   implicit many-to-many relation table columns and the data in them to
    //   keep their meaning.
    // - In one-to-one and one-to-many relations, model A is the one carrying
    //   the referencing information and possible constraint. For example, on a
    //   SQL database, model A would correspond to the table with the foreign
    //   key constraint, while model B would correspond to the table referenced
    //   by the foreign key.

    // Storage
    relations_storage: Vec<Relation<'ast>>,

    // Indexes for efficient querying.
    //
    // Why BTreeSets?
    //
    // - We can't use a BTreeMap because there can be more than one relation
    //   between two models
    // - We use a BTree because we want range queries. Meaning that with a
    //   BTreeSet, we can efficiently ask:
    //   - Give me all the relations on other models that point to this model
    //   - Give me all the relations on this model that point to other models
    //
    // Where "on this model" doesn't mean "the relation field is on the model"
    // but "the foreigr key is on this model" (= this model is model a)

    // (model_a, model_b, relation_idx)
    forward: BTreeSet<(ast::ModelId, ast::ModelId, usize)>,
    // (model_b, model_a, relation_idx)
    back: BTreeSet<(ast::ModelId, ast::ModelId, usize)>,
}

impl<'ast> Relations<'ast> {
    /// Iterator over (referenced_model_id, relation)
    #[allow(dead_code)] // not used _yet_
    pub(super) fn relations_from_model(
        &self,
        model: ast::ModelId,
    ) -> impl Iterator<Item = (ast::ModelId, &Relation<'ast>)> + '_ {
        self.forward
            .range((model, ast::ModelId::ZERO, 0)..(model, ast::ModelId::MAX, usize::MAX))
            .map(move |(_model_a_id, model_b_id, relation_idx)| (*model_b_id, &self.relations_storage[*relation_idx]))
    }

    /// Iterator over (referencing_model_id, relation)
    #[allow(dead_code)] // not used _yet_
    pub(super) fn relations_to_model(
        &self,
        model: ast::ModelId,
    ) -> impl Iterator<Item = (ast::ModelId, &Relation<'ast>)> {
        self.back
            .range((model, ast::ModelId::ZERO, 0)..(model, ast::ModelId::MAX, usize::MAX))
            .map(move |(_model_b_id, model_a_id, relation_idx)| (*model_a_id, &self.relations_storage[*relation_idx]))
    }
}

pub(super) fn infer_relations(ctx: &mut Context<'_>) {
    let mut relations = Relations::default();

    for rf in ctx.db.types.relation_fields.iter() {
        let evidence = relation_evidence(rf, ctx);
        ingest_relation(evidence, &mut relations);
    }

    ctx.db.relations = relations;
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
    ctx: &'db Context<'ast>,
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

fn ingest_relation<'ast, 'db>(evidence: RelationEvidence<'ast, 'db>, relations: &mut Relations<'ast>) {
    let relation_type = match (evidence.ast_field.arity, evidence.opposite_relation_field) {
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

            RelationType::ImplicitManyToMany {
                field_a: evidence.field_id,
                field_b: opp_field_id,
            }
        }
        (ast::FieldArity::List, Some(_)) => {
            // This is a 1:m relation defined on both sides. We skip the virtual side.
            return;
        }
        (ast::FieldArity::List, None) => {
            // This is a 1:m relation defined on the virtual side only.

            RelationType::OneToMany(OneToManyRelationFields::Back(evidence.field_id))
        }
        (ast::FieldArity::Optional | ast::FieldArity::Required, Some((opp_field_id, opp_field, _)))
            if opp_field.arity.is_required() || opp_field.arity.is_optional() =>
        {
            // This is a 1:1 relation.
            RelationType::OneToOne {
                field_a: evidence.field_id,
                field_b: opp_field_id,
            }
        }
        (ast::FieldArity::Optional | ast::FieldArity::Required, None) => {
            // This is a 1:m relation defined on the concrete side only.
            RelationType::OneToMany(OneToManyRelationFields::Forward(evidence.field_id))
        }
        (ast::FieldArity::Optional | ast::FieldArity::Required, Some((opp_field_id, _, _))) => {
            // This is a 1:m relation defined on both sides.
            RelationType::OneToMany(OneToManyRelationFields::Both(evidence.field_id, opp_field_id))
        }
    };

    let relation = Relation {
        tpe: relation_type,
        relation_name: evidence.relation_field.name,
    };

    let relation_idx = relations.relations_storage.len();
    relations.relations_storage.push(relation);
    relations.forward.insert((
        evidence.model_id,
        evidence.relation_field.referenced_model,
        relation_idx,
    ));
    relations.back.insert((
        evidence.relation_field.referenced_model,
        evidence.model_id,
        relation_idx,
    ));
}
