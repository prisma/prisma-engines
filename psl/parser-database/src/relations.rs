use crate::{
    ast::{self, WithName},
    interner::StringId,
    walkers::RelationFieldId,
    DatamodelError, Diagnostics,
    {context::Context, types::RelationField},
};
use enumflags2::bitflags;
use std::{
    collections::{BTreeSet, HashMap},
    fmt,
};

/// Detect relation types and construct relation objects to the database.
pub(super) fn infer_relations(ctx: &mut Context<'_>) {
    let mut relations = Relations::default();

    for rf in ctx.types.iter_relation_fields() {
        let evidence = relation_evidence(rf, ctx);
        ingest_relation(evidence, &mut relations, ctx);
    }

    let _ = std::mem::replace(ctx.relations, relations);
}

/// Identifier for a single relation in a Prisma schema.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub struct RelationId(u32);

impl RelationId {
    const MAX: RelationId = RelationId(u32::MAX);
    const MIN: RelationId = RelationId(u32::MIN);
}

/// Identifier for a single implicit many-to-many relation in a Prisma schema.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub struct ManyToManyRelationId(pub(crate) RelationId);

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
pub(crate) struct Relations {
    /// Storage. Private. Do not use directly.
    relations_storage: Vec<Relation>,

    /// Which field belongs to which relation. Secondary index for optimization after an observed
    /// performance regression in schema-builder.
    fields: HashMap<RelationFieldId, RelationId>,

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
    forward: BTreeSet<(ast::ModelId, ast::ModelId, RelationId)>,
    /// (model_b, model_a, relation_idx)
    ///
    /// This can be interpreted as the relations _to_ a model.
    back: BTreeSet<(ast::ModelId, ast::ModelId, RelationId)>,
}

impl std::ops::Index<RelationId> for Relations {
    type Output = Relation;

    fn index(&self, index: RelationId) -> &Self::Output {
        &self.relations_storage[index.0 as usize]
    }
}

impl std::ops::Index<RelationFieldId> for Relations {
    type Output = RelationId;

    fn index(&self, index: RelationFieldId) -> &Self::Output {
        &self.fields[&index]
    }
}

impl Relations {
    /// Iterate over all relations in the schema.
    pub(crate) fn iter(&self) -> impl ExactSizeIterator<Item = RelationId> + Clone {
        (0..self.relations_storage.len()).map(|idx| RelationId(idx as u32))
    }

    /// Iterator over all the relations in a schema.
    ///
    /// (model_a_id, model_b_id, relation)
    pub(crate) fn iter_relations(&self) -> impl Iterator<Item = (&Relation, RelationId)> + '_ {
        self.relations_storage
            .iter()
            .enumerate()
            .map(|(idx, rel)| (rel, RelationId(idx as u32)))
    }

    /// Iterator over relations where the provided model is model A, or the forward side of the
    /// relation.
    #[allow(clippy::wrong_self_convention)] // this is the name we want
    pub(crate) fn from_model(&self, model_a_id: ast::ModelId) -> impl Iterator<Item = RelationId> + '_ {
        self.forward
            .range((model_a_id, ast::ModelId::ZERO, RelationId::MIN)..(model_a_id, ast::ModelId::MAX, RelationId::MAX))
            .map(move |(_, _, relation_id)| *relation_id)
    }

    /// Iterator over relationss where the provided model is model B, or the backrelation side of
    /// the relation.
    pub(crate) fn to_model(&self, model_a_id: ast::ModelId) -> impl Iterator<Item = RelationId> + '_ {
        self.back
            .range((model_a_id, ast::ModelId::ZERO, RelationId::MIN)..(model_a_id, ast::ModelId::MAX, RelationId::MAX))
            .map(move |(_, _, relation_id)| *relation_id)
    }
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug)]
pub(super) enum OneToManyRelationFields {
    Forward(RelationFieldId),
    Back(RelationFieldId),
    Both(RelationFieldId, RelationFieldId),
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug)]
pub(super) enum OneToOneRelationFields {
    Forward(RelationFieldId),
    Both(RelationFieldId, RelationFieldId),
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug)]
pub(super) enum RelationAttributes {
    ImplicitManyToMany {
        field_a: RelationFieldId,
        field_b: RelationFieldId,
    },
    TwoWayEmbeddedManyToMany {
        field_a: RelationFieldId,
        field_b: RelationFieldId,
    },
    OneToOne(OneToOneRelationFields),
    OneToMany(OneToManyRelationFields),
}

impl RelationAttributes {
    pub(crate) fn fields(&self) -> (Option<RelationFieldId>, Option<RelationFieldId>) {
        match self {
            RelationAttributes::ImplicitManyToMany { field_a, field_b }
            | RelationAttributes::TwoWayEmbeddedManyToMany { field_a, field_b }
            | RelationAttributes::OneToOne(OneToOneRelationFields::Both(field_a, field_b))
            | RelationAttributes::OneToMany(OneToManyRelationFields::Both(field_a, field_b)) => {
                (Some(*field_a), Some(*field_b))
            }
            RelationAttributes::OneToMany(OneToManyRelationFields::Forward(field_a))
            | RelationAttributes::OneToOne(OneToOneRelationFields::Forward(field_a)) => (Some(*field_a), None),
            RelationAttributes::OneToMany(OneToManyRelationFields::Back(field_b)) => (None, Some(*field_b)),
        }
    }
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Debug)]
pub(crate) struct Relation {
    /// The `name` argument in `@relation`.
    pub(super) relation_name: Option<StringId>,
    pub(super) attributes: RelationAttributes,
    pub(super) model_a: ast::ModelId,
    pub(super) model_b: ast::ModelId,
}

impl Relation {
    pub(crate) fn is_implicit_many_to_many(&self) -> bool {
        matches!(self.attributes, RelationAttributes::ImplicitManyToMany { .. })
    }

    pub(crate) fn as_complete_fields(&self) -> Option<(RelationFieldId, RelationFieldId)> {
        match &self.attributes {
            RelationAttributes::ImplicitManyToMany { field_a, field_b } => Some((*field_a, *field_b)),
            RelationAttributes::TwoWayEmbeddedManyToMany { field_a, field_b } => Some((*field_a, *field_b)),
            RelationAttributes::OneToOne(OneToOneRelationFields::Both(field_a, field_b)) => Some((*field_a, *field_b)),
            RelationAttributes::OneToMany(OneToManyRelationFields::Both(field_a, field_b)) => {
                Some((*field_a, *field_b))
            }
            _ => None,
        }
    }

    pub(crate) fn is_two_way_embedded_many_to_many(&self) -> bool {
        matches!(self.attributes, RelationAttributes::TwoWayEmbeddedManyToMany { .. })
    }
}

// Implementation detail for this module. Should stay private.
pub(super) struct RelationEvidence<'db> {
    pub(super) ast_model: &'db ast::Model,
    pub(super) model_id: ast::ModelId,
    pub(super) ast_field: &'db ast::Field,
    pub(super) field_id: RelationFieldId,
    pub(super) is_self_relation: bool,
    pub(super) is_two_way_embedded_many_to_many_relation: bool,
    pub(super) relation_field: &'db RelationField,
    pub(super) opposite_model: &'db ast::Model,
    pub(super) opposite_relation_field: Option<(RelationFieldId, &'db ast::Field, &'db RelationField)>,
}

pub(super) fn relation_evidence<'db>(
    (relation_field_id, relation_field): (RelationFieldId, &'db RelationField),
    ctx: &'db Context<'db>,
) -> RelationEvidence<'db> {
    let ast = ctx.ast;
    let ast_model = &ast[relation_field.model_id];
    let ast_field = &ast_model[relation_field.field_id];
    let opposite_model = &ast[relation_field.referenced_model];
    let is_self_relation = relation_field.model_id == relation_field.referenced_model;
    let opposite_relation_field: Option<(RelationFieldId, &ast::Field, &'db RelationField)> = ctx
        .types
        .range_model_relation_fields(relation_field.referenced_model)
        // Only considers relations between the same models
        .filter(|(_, opposite_relation_field)| opposite_relation_field.referenced_model == relation_field.model_id)
        // Filter out the field itself, in case of self-relations
        .filter(|(_, opposite_relation_field)| {
            !is_self_relation || opposite_relation_field.field_id != relation_field.field_id
        })
        .find(|(_, opposite_relation_field)| opposite_relation_field.name == relation_field.name)
        .map(|(opp_field_id, opp_rf)| (opp_field_id, &ast[opp_rf.model_id][opp_rf.field_id], opp_rf));

    let is_two_way_embedded_many_to_many_relation = match (relation_field, opposite_relation_field) {
        (left, Some((_, _, right))) => left.fields.is_some() || right.fields.is_some(),
        _ => false,
    };

    RelationEvidence {
        ast_model,
        model_id: relation_field.model_id,
        ast_field,
        field_id: relation_field_id,
        relation_field,
        opposite_model,
        is_self_relation,
        opposite_relation_field,
        is_two_way_embedded_many_to_many_relation,
    }
}

pub(super) fn ingest_relation<'db>(evidence: RelationEvidence<'db>, relations: &mut Relations, ctx: &Context<'db>) {
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
            if evidence.ast_model.name() > evidence.opposite_model.name() {
                return;
            }

            // For self-relations, the ordering logic is different: model A
            // and model B are the same. The lexicographical order is on field names.
            if evidence.is_self_relation && evidence.ast_field.name() > opp_field.name() {
                return;
            }

            if evidence.is_two_way_embedded_many_to_many_relation {
                RelationAttributes::TwoWayEmbeddedManyToMany {
                    field_a: evidence.field_id,
                    field_b: opp_field_id,
                }
            } else {
                RelationAttributes::ImplicitManyToMany {
                    field_a: evidence.field_id,
                    field_b: opp_field_id,
                }
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

            if [evidence.ast_model.name(), evidence.ast_field.name()]
                > [evidence.opposite_model.name(), opp_field.name()]
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
                if [evidence.ast_model.name(), evidence.ast_field.name()]
                    > [evidence.opposite_model.name(), opp_field.name()]
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
                Some(fields) => {
                    let fields_are_unique =
                        ctx.types.model_attributes[&evidence.model_id]
                            .ast_indexes
                            .iter()
                            .any(|(_, idx)| {
                                idx.is_unique() && idx.fields.len() == fields.len() && {
                                    idx.fields
                                        .iter()
                                        .zip(fields.iter())
                                        .all(|(idx_field, field)| matches!(idx_field.path.field_in_index(), either::Either::Left(id) if id == *field))
                                }
                            });
                    if fields_are_unique {
                        RelationAttributes::OneToOne(OneToOneRelationFields::Forward(evidence.field_id))
                    } else {
                        RelationAttributes::OneToMany(OneToManyRelationFields::Forward(evidence.field_id))
                    }
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

    let relation_id = RelationId(relations.relations_storage.len() as u32);

    relations.relations_storage.push(relation);
    relations.fields.insert(evidence.field_id, relation_id);
    if let Some((opposite_field_id, _, _)) = evidence.opposite_relation_field {
        relations.fields.insert(opposite_field_id, relation_id);
    }

    relations
        .forward
        .insert((evidence.model_id, evidence.relation_field.referenced_model, relation_id));

    relations
        .back
        .insert((evidence.relation_field.referenced_model, evidence.model_id, relation_id));
}

/// An action describing the way referential integrity is managed in the system.
///
/// An action is triggered when a relation constraint gets violated in a way
/// that would make the the data inconsistent, e.g. deleting or updating a
/// referencing record that leaves related records into a wrong state.
///
/// ```ignore
/// @relation(fields: [a], references: [b], onDelete: NoAction, onUpdate: Cascade)
///                                                   ^^^^^^^^            ^^^^^^^
/// ```
#[repr(u8)]
#[bitflags]
#[derive(Debug, Copy, PartialEq, Clone)]
pub enum ReferentialAction {
    /// Deletes record if dependent record is deleted. Updates relation scalar
    /// fields if referenced scalar fields of the dependent record are updated.
    ///
    /// Prevents operation (both updates and deletes) from succeeding if any
    /// records are connected.
    Cascade,
    /// Prevents operation (both updates and deletes) from succeeding if any
    /// records are connected. This behavior will always result in a runtime
    /// error for required relations.
    Restrict,
    /// Behavior is database specific. Either defers throwing an integrity check
    /// error until the end of the transaction or errors immediately. If
    /// deferred, this makes it possible to temporarily violate integrity in a
    /// transaction while making sure that subsequent operations in the
    /// transaction restore integrity.
    ///
    /// When using relationMode = "prisma", NoAction becomes an alias of
    /// the emulated Restrict (when supported).
    NoAction,
    /// Sets relation scalar fields to null if the relation is deleted or
    /// updated. This will always result in a runtime error if one or more of the
    /// relation scalar fields are required.
    SetNull,
    /// Sets relation scalar fields to their default values on update or delete
    /// of relation. Will always result in a runtime error if no defaults are
    /// provided for any relation scalar fields.
    SetDefault,
}

impl ReferentialAction {
    /// True if the action modifies the related items.
    pub fn triggers_modification(self) -> bool {
        !matches!(self, Self::NoAction | Self::Restrict)
    }

    /// The string representation of the referential action in the schema.
    pub fn as_str(self) -> &'static str {
        match self {
            ReferentialAction::Cascade => "Cascade",
            ReferentialAction::Restrict => "Restrict",
            ReferentialAction::NoAction => "NoAction",
            ReferentialAction::SetNull => "SetNull",
            ReferentialAction::SetDefault => "SetDefault",
        }
    }

    /// The documentation string to display in autocompletion / editor hints.
    pub fn documentation(&self) -> &'static str {
        match self {
            ReferentialAction::Cascade => "Delete the child records when the parent record is deleted.",
            ReferentialAction::Restrict => "Prevent deleting a parent record as long as it is referenced.",
            ReferentialAction::NoAction => "Prevent deleting a parent record as long as it is referenced.",
            ReferentialAction::SetNull => "Set the referencing fields to NULL when the referenced record is deleted.",
            ReferentialAction::SetDefault => {
                "Set the referencing field's value to the default when the referenced record is deleted."
            }
        }
    }

    pub(crate) fn try_from_expression(
        expr: &ast::Expression,
        diagnostics: &mut Diagnostics,
    ) -> Option<ReferentialAction> {
        match crate::coerce::constant(expr, diagnostics)? {
            "Cascade" => Some(ReferentialAction::Cascade),
            "Restrict" => Some(ReferentialAction::Restrict),
            "NoAction" => Some(ReferentialAction::NoAction),
            "SetNull" => Some(ReferentialAction::SetNull),
            "SetDefault" => Some(ReferentialAction::SetDefault),
            s => {
                let message = format!("Invalid referential action: `{s}`");

                diagnostics.push_error(DatamodelError::new_attribute_validation_error(
                    &message,
                    "@relation",
                    expr.span(),
                ));

                None
            }
        }
    }
}

impl AsRef<str> for ReferentialAction {
    fn as_ref(&self) -> &'static str {
        match self {
            ReferentialAction::Cascade => "Cascade",
            ReferentialAction::Restrict => "Restrict",
            ReferentialAction::NoAction => "NoAction",
            ReferentialAction::SetNull => "SetNull",
            ReferentialAction::SetDefault => "SetDefault",
        }
    }
}

impl fmt::Display for ReferentialAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}
