use crate::{ast, relations::*, walkers::*, ParserDatabase, ScalarFieldType};

/// A relation that has the minimal amount of information for us to create one. Useful for
/// validation purposes. Holds all possible relation types.
pub type RelationWalker<'ast, 'db> = Walker<'ast, 'db, RelationId>;

impl<'ast, 'db> RelationWalker<'ast, 'db> {
    /// Converts the walker to either an implicit many to many, or a inline relation walker
    /// gathering 1:1 and 1:n relations.
    pub fn refine(self) -> RefinedRelationWalker<'ast, 'db> {
        if self.get().is_many_to_many() {
            RefinedRelationWalker::ImplicitManyToMany(ImplicitManyToManyRelationWalker(self))
        } else {
            RefinedRelationWalker::Inline(InlineRelationWalker(self))
        }
    }

    pub(crate) fn has_field(self, model_id: ast::ModelId, field_id: ast::FieldId) -> bool {
        self.get().has_field(model_id, field_id)
    }

    /// The relation attributes parsed from the AST.
    fn get(self) -> &'db Relation {
        &self.db.relations[self.id]
    }
}

/// Splits the relation to different types.
pub enum RefinedRelationWalker<'ast, 'db> {
    /// 1:1 and 1:n relations, where one side defines the relation arguments.
    Inline(InlineRelationWalker<'ast, 'db>),
    /// Implicit m:n relation. The arguments are inferred by Prisma.
    ImplicitManyToMany(ImplicitManyToManyRelationWalker<'ast, 'db>),
}

impl<'ast, 'db> RefinedRelationWalker<'ast, 'db> {
    /// Try interpreting this relation as an inline (1:n or 1:1 — without join table) relation.
    pub fn as_inline(&self) -> Option<InlineRelationWalker<'ast, 'db>> {
        match self {
            RefinedRelationWalker::Inline(inline) => Some(*inline),
            _ => None,
        }
    }

    /// Try interpreting this relation as an implicit many-to-many relation.
    pub fn as_many_to_many(&self) -> Option<ImplicitManyToManyRelationWalker<'ast, 'db>> {
        match self {
            RefinedRelationWalker::ImplicitManyToMany(m2m) => Some(*m2m),
            _ => None,
        }
    }
}

/// A scalar inferred by loose/magic reformatting.
#[allow(missing_docs)]
pub struct InferredField<'ast, 'db> {
    pub name: String,
    pub arity: ast::FieldArity,
    pub tpe: ScalarFieldType,
    pub blueprint: ScalarFieldWalker<'ast, 'db>,
}

/// The scalar fields on the concrete side of the relation.
pub enum ReferencingFields<'ast, 'db> {
    /// Existing scalar fields
    Concrete(Box<dyn ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>> + 'db>),
    /// Inferred scalar fields
    Inferred(Vec<InferredField<'ast, 'db>>),
    /// Error
    NA,
}

/// An explicitly defined 1:1 or 1:n relation. The walker has the referencing side defined, but
/// might miss the back relation in the AST.
#[derive(Copy, Clone)]
pub struct InlineRelationWalker<'ast, 'db>(RelationWalker<'ast, 'db>);

impl<'ast, 'db> InlineRelationWalker<'ast, 'db> {
    /// Get the relation attributes defined in the AST.
    fn get(self) -> &'db Relation {
        &self.0.db.relations[self.0.id]
    }

    /// The relation is 1:1, having at most one record on both sides of the relation.
    pub fn is_one_to_one(self) -> bool {
        matches!(self.get().attributes, RelationAttributes::OneToOne(_))
    }

    /// The model which holds the relation arguments.
    pub fn referencing_model(self) -> ModelWalker<'ast, 'db> {
        self.0.db.walk_model(self.get().model_a)
    }

    /// The model referenced and which hold the back-relation field.
    pub fn referenced_model(self) -> ModelWalker<'ast, 'db> {
        self.0.db.walk_model(self.get().model_b)
    }

    /// If the relation is defined from both sides, convert to an explicit relation
    /// walker.
    pub fn as_complete(self) -> Option<CompleteInlineRelationWalker<'ast, 'db>> {
        match (self.forward_relation_field(), self.back_relation_field()) {
            (Some(field_a), Some(field_b)) => {
                let walker = CompleteInlineRelationWalker {
                    side_a: (self.referencing_model().model_id, field_a.field_id),
                    side_b: (self.referenced_model().model_id, field_b.field_id),
                    db: self.0.db,
                };

                Some(walker)
            }
            _ => None,
        }
    }

    /// Should only be used for lifting. The referencing fields (including possibly inferred ones).
    pub fn referencing_fields(self) -> ReferencingFields<'ast, 'db> {
        self.forward_relation_field()
            .and_then(|rf| rf.fields())
            .map(|fields| ReferencingFields::Concrete(Box::new(fields)))
            .unwrap_or_else(|| match self.referenced_model().unique_criterias().next() {
                Some(first_unique_criteria) => ReferencingFields::Inferred(
                    first_unique_criteria
                        .fields()
                        .map(|field| {
                            let name = format!(
                                "{}{}",
                                camel_case(self.referenced_model().name()),
                                pascal_case(field.name())
                            );

                            if let Some(existing_field) =
                                self.referencing_model().scalar_fields().find(|sf| sf.name() == name)
                            {
                                InferredField {
                                    name,
                                    arity: existing_field.ast_field().arity,
                                    tpe: existing_field.scalar_field_type(),
                                    blueprint: field,
                                }
                            } else {
                                InferredField {
                                    name,
                                    arity: ast::FieldArity::Optional,
                                    tpe: field.scalar_field_type(),
                                    blueprint: field,
                                }
                            }
                        })
                        .collect(),
                ),
                None => ReferencingFields::NA,
            })
    }

    /// Should only be used for lifting. The referenced fields. Inferred or specified.
    pub fn referenced_fields(self) -> Box<dyn Iterator<Item = ScalarFieldWalker<'ast, 'db>> + 'db> {
        self.forward_relation_field()
            .and_then(
                |field: RelationFieldWalker<'ast, 'db>| -> Option<Box<dyn Iterator<Item = ScalarFieldWalker<'ast, 'db>>>> {
                    field.referenced_fields().map(|fields| Box::new(fields) as Box<dyn Iterator<Item = ScalarFieldWalker<'ast, 'db>>>)
                },
            )
            .unwrap_or_else(move || {
                Box::new(
                    self.referenced_model()
                        .unique_criterias()
                        .find(|c| c.is_strict_criteria())
                        .into_iter()
                        .flat_map(|c| c.fields()),
                )
            })
    }

    /// The forward relation field (the relation field on model A, the referencing model).
    pub fn forward_relation_field(self) -> Option<RelationFieldWalker<'ast, 'db>> {
        let model = self.referencing_model();
        match self.get().attributes {
            RelationAttributes::OneToOne(OneToOneRelationFields::Forward(a))
            | RelationAttributes::OneToOne(OneToOneRelationFields::Both(a, _))
            | RelationAttributes::OneToMany(OneToManyRelationFields::Both(a, _))
            | RelationAttributes::OneToMany(OneToManyRelationFields::Forward(a)) => Some(model.relation_field(a)),
            RelationAttributes::OneToMany(OneToManyRelationFields::Back(_)) => None,
            RelationAttributes::ImplicitManyToMany { field_a: _, field_b: _ } => unreachable!(),
        }
    }

    /// The arity of the forward relation field.
    pub fn forward_relation_field_arity(self) -> ast::FieldArity {
        self.forward_relation_field()
            .map(|rf| rf.ast_field().arity)
            .unwrap_or_else(|| {
                let is_required = match self.referencing_fields() {
                    ReferencingFields::Concrete(mut fields) => fields.all(|f| f.ast_field().arity.is_required()),
                    ReferencingFields::Inferred(fields) => fields.iter().all(|f| f.arity.is_required()),
                    ReferencingFields::NA => todo!(),
                };
                if is_required {
                    ast::FieldArity::Required
                } else {
                    ast::FieldArity::Optional
                }
            })
    }

    /// The contents of the `map: ...` argument of the `@relation` attribute.
    pub fn mapped_name(self) -> Option<&'db str> {
        self.forward_relation_field().and_then(|field| field.mapped_name())
    }

    /// The back relation field, or virtual relation field (on model B, the referenced model).
    pub fn back_relation_field(self) -> Option<RelationFieldWalker<'ast, 'db>> {
        let model = self.referenced_model();
        match self.get().attributes {
            RelationAttributes::OneToOne(OneToOneRelationFields::Both(_, b))
            | RelationAttributes::OneToMany(OneToManyRelationFields::Both(_, b))
            | RelationAttributes::OneToMany(OneToManyRelationFields::Back(b)) => Some(model.relation_field(b)),
            RelationAttributes::OneToMany(OneToManyRelationFields::Forward(_))
            | RelationAttributes::OneToOne(OneToOneRelationFields::Forward(_)) => None,
            RelationAttributes::ImplicitManyToMany { field_a: _, field_b: _ } => unreachable!(),
        }
    }

    /// The name of the relation. Either uses the `name` (or default) argument,
    /// or generates an implicit name.
    pub fn relation_name(self) -> RelationName<'db> {
        self.get()
            .relation_name
            .as_ref()
            .map(|s| RelationName::Explicit(self.0.db.resolve_str(s)))
            .unwrap_or_else(|| RelationName::generated(self.referencing_model().name(), self.referenced_model().name()))
    }
}

/// Describes an implicit m:n relation between two models. Neither side defines fields, attributes
/// or referential actions, which are all inferred by Prisma.
#[derive(Copy, Clone)]
pub struct ImplicitManyToManyRelationWalker<'ast, 'db>(RelationWalker<'ast, 'db>);

impl<'ast, 'db> ImplicitManyToManyRelationWalker<'ast, 'db> {
    /// Gets the relation attributes from the AST.
    fn get(&self) -> &'db Relation {
        &self.0.db.relations[self.0.id]
    }

    /// The model which comes first in the alphabetical order.
    pub fn model_a(self) -> ModelWalker<'ast, 'db> {
        self.0.db.walk_model(self.get().model_a)
    }

    /// The model which comes after model a in the alphabetical order.
    pub fn model_b(self) -> ModelWalker<'ast, 'db> {
        self.0.db.walk_model(self.get().model_b)
    }

    /// The field that defines the relation in model a.
    pub fn field_a(self) -> RelationFieldWalker<'ast, 'db> {
        match self.get().attributes {
            RelationAttributes::ImplicitManyToMany { field_a, field_b: _ } => self.model_a().relation_field(field_a),
            _ => unreachable!(),
        }
    }

    /// The field that defines the relation in model b.
    pub fn field_b(self) -> RelationFieldWalker<'ast, 'db> {
        match self.get().attributes {
            RelationAttributes::ImplicitManyToMany { field_a: _, field_b } => self.model_b().relation_field(field_b),
            _ => unreachable!(),
        }
    }

    /// The name of the relation.
    pub fn relation_name(self) -> RelationName<'db> {
        self.field_a().relation_name()
    }
}

/// Represents a relation that has fields and references defined in one of the
/// relation fields. Includes 1:1 and 1:n relations that are defined from both sides.
#[derive(Copy, Clone)]
pub struct CompleteInlineRelationWalker<'ast, 'db> {
    pub(crate) side_a: (ast::ModelId, ast::FieldId),
    pub(crate) side_b: (ast::ModelId, ast::FieldId),
    pub(crate) db: &'db ParserDatabase<'ast>,
}

#[allow(missing_docs)]
impl<'ast, 'db> CompleteInlineRelationWalker<'ast, 'db> {
    /// The model that defines the relation fields and actions.
    pub fn referencing_model(self) -> ModelWalker<'ast, 'db> {
        ModelWalker {
            model_id: self.side_a.0,
            db: self.db,
            model_attributes: &self.db.types.model_attributes[&self.side_a.0],
        }
    }

    /// The implicit relation side.
    pub fn referenced_model(self) -> ModelWalker<'ast, 'db> {
        ModelWalker {
            model_id: self.side_b.0,
            db: self.db,
            model_attributes: &self.db.types.model_attributes[&self.side_b.0],
        }
    }

    pub fn referencing_field(self) -> RelationFieldWalker<'ast, 'db> {
        RelationFieldWalker {
            model_id: self.side_a.0,
            field_id: self.side_a.1,
            db: self.db,
            relation_field: &self.db.types.relation_fields[&(self.side_a.0, self.side_a.1)],
        }
    }

    pub fn referenced_field(self) -> RelationFieldWalker<'ast, 'db> {
        RelationFieldWalker {
            model_id: self.side_b.0,
            field_id: self.side_b.1,
            db: self.db,
            relation_field: &self.db.types.relation_fields[&(self.side_b.0, self.side_b.1)],
        }
    }

    /// The scalar fields defining the relation on the referenced model.
    pub fn referenced_fields(self) -> impl ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>> + 'db {
        let f = move |field_id: &ast::FieldId| {
            let model_id = self.referenced_model().model_id;

            ScalarFieldWalker {
                model_id,
                field_id: *field_id,
                db: self.db,
                scalar_field: &self.db.types.scalar_fields[&(model_id, *field_id)],
            }
        };

        match self.referencing_field().relation_field.references.as_ref() {
            Some(references) => references.iter().map(f),
            None => [].iter().map(f),
        }
    }

    /// The scalar fields on the defining the relation on the referencing model.
    pub fn referencing_fields(self) -> impl ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>> + 'db {
        let f = move |field_id: &ast::FieldId| {
            let model_id = self.referencing_model().model_id;

            ScalarFieldWalker {
                model_id,
                field_id: *field_id,
                db: self.db,
                scalar_field: &self.db.types.scalar_fields[&(model_id, *field_id)],
            }
        };

        match self.referencing_field().relation_field.fields.as_ref() {
            Some(references) => references.iter().map(f),
            None => [].iter().map(f),
        }
    }

    /// Gives the onUpdate referential action of the relation. If not defined
    /// explicitly, returns the default value.
    pub fn on_update(self) -> ReferentialAction {
        use ReferentialAction::*;

        self.referencing_field().explicit_on_update().unwrap_or(Cascade)
    }

    /// Prisma allows setting the relation field as optional, even if one of the
    /// underlying scalar fields is required. For the purpose of referential
    /// actions, we count the relation field required if any of the underlying
    /// fields is required.
    pub fn referential_arity(self) -> ast::FieldArity {
        self.referencing_field().referential_arity()
    }
}

fn pascal_case(input: &str) -> String {
    let mut c = input.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn camel_case(input: &str) -> String {
    let mut c = input.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
    }
}
