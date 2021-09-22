use std::collections::BTreeSet;

use crate::{
    ast,
    transform::ast_to_dml::db::{context::Context, types::RelationField},
};

/// Storage for the relations in a schema.
///
/// A relation is always between two models. One model is assigned the role
/// of "model A", and the other is "model B". The meaning of "model A" and
/// "model B" depends on the type of relation.
///
/// - In implicit many-to-many relations, model A and model B are ordered
///   lexicographically, by model name, and failing that by relation field
///   name. This order must be stable in order for the columns in the
///   implicit many-to-many relation table columns and the data in them to
///   keep their meaning.
/// - In one-to-one and one-to-many relations, model A is the one carrying
///   the referencing information and possible constraint. For example, on a
///   SQL database, model A would correspond to the table with the foreign
///   key constraint, while model B would correspond to the table referenced
///   by the foreign key.
#[derive(Debug, Default)]
pub(crate) struct Relations<'ast> {
    /// Storage. Private. Do not use directly.
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
    // but "the foreign key is on this model" (= this model is model a)
    /// (model_a, model_b, relation_idx)
    ///
    /// This can be interpreted as the relations _from_ a model.
    forward: BTreeSet<(ast::ModelId, ast::ModelId, usize)>,
    /// (model_b, model_a, relation_idx)
    ///
    /// This can be interpreted as the relations _to_ a model.
    back: BTreeSet<(ast::ModelId, ast::ModelId, usize)>,
}

impl<'ast> Relations<'ast> {
    /// Iterator over all the relations in a schema.
    ///
    /// (model_a_id, model_b_id, relation)
    #[allow(dead_code)] // not used _yet_
    pub(crate) fn iter_relations(&self) -> impl Iterator<Item = (ast::ModelId, ast::ModelId, &Relation<'ast>)> + '_ {
        self.forward.iter().map(move |(model_a_id, model_b_id, relation_idx)| {
            (*model_a_id, *model_b_id, &self.relations_storage[*relation_idx])
        })
    }

    /// Iterator over (model_b_id, relation)
    #[allow(dead_code)] // not used _yet_
    pub(crate) fn relations_from_model(
        &self,
        model_a_id: ast::ModelId,
    ) -> impl Iterator<Item = (ast::ModelId, &Relation<'ast>)> + '_ {
        self.forward
            .range((model_a_id, ast::ModelId::ZERO, 0)..(model_a_id, ast::ModelId::MAX, usize::MAX))
            .map(move |(_model_a_id, model_b_id, relation_idx)| (*model_b_id, &self.relations_storage[*relation_idx]))
    }

    /// Iterator over (model_a_id, relation)
    #[allow(dead_code)] // not used _yet_
    pub(crate) fn relations_to_model(
        &self,
        model_b_id: ast::ModelId,
    ) -> impl Iterator<Item = (ast::ModelId, &Relation<'ast>)> {
        self.back
            .range((model_b_id, ast::ModelId::ZERO, 0)..(model_b_id, ast::ModelId::MAX, usize::MAX))
            .map(move |(_model_b_id, model_a_id, relation_idx)| (*model_a_id, &self.relations_storage[*relation_idx]))
    }
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug)]
pub(super) enum OneToManyRelationFields {
    Forward(ast::FieldId),
    Back(ast::FieldId),
    Both(ast::FieldId, ast::FieldId),
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug)]
pub(super) enum OneToOneRelationFields {
    Forward(ast::FieldId),
    Both(ast::FieldId, ast::FieldId),
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug)]
pub(super) enum RelationType {
    ImplicitManyToMany {
        field_a: ast::FieldId,
        field_b: ast::FieldId,
    },
    OneToOne(OneToOneRelationFields),
    OneToMany(OneToManyRelationFields),
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug)]
pub(crate) struct Relation<'ast> {
    /// The `name` argument in `@relation`.
    relation_name: Option<&'ast str>,
    r#type: RelationType,
}

impl<'ast> Relation<'ast> {
    pub(crate) fn is_many_to_many(&self) -> bool {
        matches!(self.r#type, RelationType::ImplicitManyToMany { .. })
    }

    pub(crate) fn fields(&self) -> Option<(ast::FieldId, ast::FieldId)> {
        match &self.r#type {
            RelationType::ImplicitManyToMany { field_a, field_b } => Some((*field_a, *field_b)),
            RelationType::OneToOne(OneToOneRelationFields::Both(field_a, field_b)) => Some((*field_a, *field_b)),
            RelationType::OneToMany(OneToManyRelationFields::Both(field_a, field_b)) => Some((*field_a, *field_b)),
            _ => None,
        }
    }
}

// Implementation detail for this module. Should stay private.
pub(super) struct RelationEvidence<'ast, 'db> {
    pub(super) ast_model: &'ast ast::Model,
    pub(super) model_id: ast::ModelId,
    pub(super) ast_field: &'ast ast::Field,
    pub(super) field_id: ast::FieldId,
    pub(super) is_self_relation: bool,
    pub(super) relation_field: &'db RelationField<'ast>,
    pub(super) opposite_model: &'ast ast::Model,
    pub(super) opposite_relation_field: Option<(ast::FieldId, &'ast ast::Field, &'db RelationField<'ast>)>,
}

pub(super) fn relation_evidence<'ast, 'db>(
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

pub(super) fn ingest_relation<'ast, 'db>(
    evidence: RelationEvidence<'ast, 'db>,
    relations: &mut Relations<'ast>,
    ctx: &'db Context<'ast>,
) {
    let relation_type = match (evidence.ast_field.arity, evidence.opposite_relation_field) {
        // m:n
        (ast::FieldArity::List, Some((opp_field_id, opp_field, _))) if opp_field.arity.is_list() => {
            // This is an implicit many-to-many relation.

            // We will meet the relation twice when we walk over all relation
            // fields, so we only instantiate it when the relation field is that
            // of model A, and the opposite is model B.
            if evidence.ast_model.name.name > evidence.opposite_model.name.name {
                return;
            }

            // For self-relations, the ordering logic is different: model A
            // and model B are the same. The lexicographical order is on field names.
            if evidence.is_self_relation && evidence.ast_field.name() > opp_field.name() {
                return;
            }

            RelationType::ImplicitManyToMany {
                field_a: evidence.field_id,
                field_b: opp_field_id,
            }
        }

        // 1:1
        (ast::FieldArity::Required, Some((opp_field_id, opp_field, _))) if opp_field.arity.is_optional() => {
            // This is a required 1:1 relation, and we are on the required side.
            RelationType::OneToOne(OneToOneRelationFields::Both(evidence.field_id, opp_field_id))
        }
        (ast::FieldArity::Optional, Some((_, opp_field, _))) if opp_field.arity.is_required() => {
            // This is a required 1:1 relation, and we are on the virtual side. Skip.
            return;
        }
        (ast::FieldArity::Required, Some((_, opp_field, _))) if opp_field.arity.is_required() => {
            // This is a 1:1 relation that is required on both sides. Error.
            return; // TODO: error
        }
        (ast::FieldArity::Optional, Some((opp_field_id, opp_field, opp_relation_field)))
            if opp_field.arity.is_optional() =>
        {
            // This is a 1:1 relation that is optional on both sides. We must infer which side is model A.

            if evidence.relation_field.fields.is_some() {
                RelationType::OneToOne(OneToOneRelationFields::Both(evidence.field_id, opp_field_id))
            } else if opp_relation_field.fields.is_some() {
                RelationType::OneToOne(OneToOneRelationFields::Both(opp_field_id, evidence.field_id))
            } else {
                return; // TODO: error on ambiguous relation
            }
        }
        // 1:m
        (ast::FieldArity::List, Some(_)) => {
            // This is a 1:m relation defined on both sides. We skip the virtual side.
            return;
        }
        (ast::FieldArity::List, None) => {
            // This is a 1:m relation defined on the virtual side only.
            RelationType::OneToMany(OneToManyRelationFields::Back(evidence.field_id))
        }
        (ast::FieldArity::Required | ast::FieldArity::Optional, Some((opp_field_id, _, _))) => {
            // This is a 1:m relation defined on both sides.
            RelationType::OneToMany(OneToManyRelationFields::Both(evidence.field_id, opp_field_id))
        }

        // 1:m or 1:1
        (ast::FieldArity::Optional | ast::FieldArity::Required, None) => {
            // This is a relation defined on both sides. We check whether the
            // relation scalar fields are unique to determine whether it is a
            // 1:1 or a 1:m relation.
            match &evidence.relation_field.fields {
                Some(fields) if ctx.db.walk_model(evidence.model_id).fields_are_unique(fields) => {
                    RelationType::OneToOne(OneToOneRelationFields::Forward(evidence.field_id))
                }
                _ => RelationType::OneToMany(OneToManyRelationFields::Forward(evidence.field_id)),
            }
        }
    };

    let relation = Relation {
        r#type: relation_type,
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
