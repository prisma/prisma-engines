use crate::{
    ast,
    transform::ast_to_dml::db::{context::Context, types::RelationField},
};
use std::collections::BTreeSet;

/// Detect relation types and construct relation objects to the database.
pub(super) fn infer_relations(ctx: &mut Context<'_>) {
    let mut relations = Relations::default();

    for rf in ctx.db.types.relation_fields.iter() {
        let evidence = relation_evidence(rf, ctx);
        ingest_relation(evidence, &mut relations, ctx);
    }

    ctx.db.relations = relations;
}

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
    pub(super) relations_storage: Vec<Relation<'ast>>,

    // Indexes for efficient querying.
    //
    // Why BTreeSets?
    //
    // - We can't use a BTreeMap because there can be more than one relation
    //   between two models.
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
    pub(crate) fn iter_relations(&self) -> impl Iterator<Item = (ast::ModelId, ast::ModelId, &Relation<'ast>)> + '_ {
        self.forward.iter().map(move |(model_a_id, model_b_id, relation_idx)| {
            (*model_a_id, *model_b_id, &self.relations_storage[*relation_idx])
        })
    }

    /// Iterator over relation id
    pub(crate) fn from_model(&self, model_a_id: ast::ModelId) -> impl Iterator<Item = usize> + '_ {
        self.forward
            .range((model_a_id, ast::ModelId::ZERO, 0)..(model_a_id, ast::ModelId::MAX, usize::MAX))
            .map(move |(_, _, relation_idx)| *relation_idx)
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
pub(super) enum RelationAttributes {
    ImplicitManyToMany {
        field_a: ast::FieldId,
        field_b: ast::FieldId,
    },
    OneToOne(OneToOneRelationFields),
    OneToMany(OneToManyRelationFields),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RelationType {
    ImplicitManyToMany,
    OneToOne,
    OneToMany,
}

impl RelationType {
    pub(crate) fn is_one_to_one(self) -> bool {
        matches!(self, Self::OneToOne)
    }
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug)]
pub(crate) struct Relation<'ast> {
    /// The `name` argument in `@relation`.
    pub(super) relation_name: Option<&'ast str>,
    pub(super) attributes: RelationAttributes,
    pub(super) model_a: ast::ModelId,
    pub(super) model_b: ast::ModelId,
}

impl<'ast> Relation<'ast> {
    pub(crate) fn is_many_to_many(&self) -> bool {
        matches!(self.attributes, RelationAttributes::ImplicitManyToMany { .. })
    }

    pub(crate) fn as_complete_fields(&self) -> Option<(ast::FieldId, ast::FieldId)> {
        match &self.attributes {
            RelationAttributes::ImplicitManyToMany { field_a, field_b } => Some((*field_a, *field_b)),
            RelationAttributes::OneToOne(OneToOneRelationFields::Both(field_a, field_b)) => Some((*field_a, *field_b)),
            RelationAttributes::OneToMany(OneToManyRelationFields::Both(field_a, field_b)) => {
                Some((*field_a, *field_b))
            }
            _ => None,
        }
    }

    pub(super) fn r#type(&self) -> RelationType {
        match self.attributes {
            RelationAttributes::ImplicitManyToMany { .. } => RelationType::ImplicitManyToMany,
            RelationAttributes::OneToOne(_) => RelationType::OneToOne,
            RelationAttributes::OneToMany(_) => RelationType::OneToMany,
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
        .walk_model(relation_field.referenced_model)
        .relation_fields()
        // Only considers relations between the same models
        .filter(|opposite_relation_field| opposite_relation_field.references_model(*model_id))
        // Filter out the field itself, in case of self-relations
        .filter(|opposite_relation_field| !is_self_relation || opposite_relation_field.field_id != *field_id)
        .find(|opposite_relation_field| opposite_relation_field.explicit_relation_name() == relation_field.name)
        .map(|opp_rf| (opp_rf.field_id(), opp_rf.ast_field(), opp_rf.relation_field));

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
    // In this function, we want to ingest the relation only once,
    // so if we know that we will create a relation for the opposite
    // field, we skip the field by returning early.
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

            RelationAttributes::ImplicitManyToMany {
                field_a: evidence.field_id,
                field_b: opp_field_id,
            }
        }

        // 1:1
        (ast::FieldArity::Required, Some((opp_field_id, opp_field, _))) if opp_field.arity.is_optional() => {
            // This is a required 1:1 relation, and we are on the required side.
            RelationAttributes::OneToOne(OneToOneRelationFields::Both(evidence.field_id, opp_field_id))
        }
        (ast::FieldArity::Required, Some((opp_field_id, opp_field, _))) if opp_field.arity.is_required() => {
            // This is a 1:1 relation that is required on both sides. We are going to reject this later,
            // so which model is model A doesn't matter.

            if [evidence.ast_model.name.name.as_str(), evidence.ast_field.name()]
                > [evidence.opposite_model.name.name.as_str(), opp_field.name()]
            {
                return;
            }

            RelationAttributes::OneToOne(OneToOneRelationFields::Both(evidence.field_id, opp_field_id))
        }
        (ast::FieldArity::Optional, Some((_, opp_field, _))) if opp_field.arity.is_required() => {
            // This is a required 1:1 relation, and we are on the virtual side. Skip.
            return;
        }
        (ast::FieldArity::Optional, Some((opp_field_id, opp_field, opp_field_attributes)))
            if opp_field.arity.is_optional() =>
        {
            // This is a 1:1 relation that is optional on both sides. We must infer which side is model A.

            if evidence.relation_field.fields.is_some() {
                RelationAttributes::OneToOne(OneToOneRelationFields::Both(evidence.field_id, opp_field_id))
            } else if opp_field_attributes.fields.is_none() {
                // No fields defined, we have to break the tie: take the first model name / field name (self relations)
                // in lexicographic order.
                if [evidence.ast_model.name.name.as_str(), evidence.ast_field.name()]
                    > [evidence.opposite_model.name.name.as_str(), opp_field.name()]
                {
                    return;
                }

                RelationAttributes::OneToOne(OneToOneRelationFields::Both(evidence.field_id, opp_field_id))
            } else {
                // Opposite field has fields, it's the forward side. Return.
                return;
            }
        }

        // 1:m
        (ast::FieldArity::List, Some(_)) => {
            // This is a 1:m relation defined on both sides. We skip the virtual side.
            return;
        }
        (ast::FieldArity::List, None) => {
            // This is a 1:m relation defined on the virtual side only.
            RelationAttributes::OneToMany(OneToManyRelationFields::Back(evidence.field_id))
        }
        (ast::FieldArity::Required | ast::FieldArity::Optional, Some((opp_field_id, _, _))) => {
            // This is a 1:m relation defined on both sides.
            RelationAttributes::OneToMany(OneToManyRelationFields::Both(evidence.field_id, opp_field_id))
        }

        // 1:m or 1:1
        (ast::FieldArity::Optional | ast::FieldArity::Required, None) => {
            // This is a relation defined on both sides. We check whether the
            // relation scalar fields are unique to determine whether it is a
            // 1:1 or a 1:m relation.
            match &evidence.relation_field.fields {
                Some(fields) if ctx.db.walk_model(evidence.model_id).fields_are_unique(fields) => {
                    RelationAttributes::OneToOne(OneToOneRelationFields::Forward(evidence.field_id))
                }
                _ => RelationAttributes::OneToMany(OneToManyRelationFields::Forward(evidence.field_id)),
            }
        }
    };

    let relation = match relation_type {
        // Back-only relation fields are special, because we always take the forward side when defining the relation type,
        // except in this case, because there is no forward side.
        RelationAttributes::OneToMany(OneToManyRelationFields::Back(_)) => Relation {
            attributes: relation_type,
            relation_name: evidence.relation_field.name,
            model_a: evidence.relation_field.referenced_model,
            model_b: evidence.model_id,
        },
        _ => Relation {
            attributes: relation_type,
            relation_name: evidence.relation_field.name,
            model_a: evidence.model_id,
            model_b: evidence.relation_field.referenced_model,
        },
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
