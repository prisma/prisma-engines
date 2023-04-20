use crate::{
    ast::{self, FieldArity},
    types::{RelationField, RelationFieldId},
    walkers::*,
    ReferentialAction,
};
use std::{borrow::Cow, fmt, hash::Hasher};

/// A relation field on a model in the schema.
pub type RelationFieldWalker<'db> = Walker<'db, RelationFieldId>;

impl<'db> RelationFieldWalker<'db> {
    /// The relation starts or ends to a view.
    pub fn one_side_is_view(self) -> bool {
        self.model().ast_model().is_view() || self.related_model().ast_model().is_view()
    }

    /// The foreign key name of the relation (`@relation(map: ...)`).
    pub fn mapped_name(self) -> Option<&'db str> {
        self.attributes().mapped_name.map(|string_id| &self.db[string_id])
    }

    /// The field name.
    pub fn name(self) -> &'db str {
        self.ast_field().name()
    }

    /// The AST node of the field.
    pub fn ast_field(self) -> &'db ast::Field {
        let RelationField { model_id, field_id, .. } = self.db.types[self.id];
        &self.db.ast[model_id][field_id]
    }

    pub(crate) fn attributes(self) -> &'db RelationField {
        &self.db.types[self.id]
    }

    /// The onDelete argument on the relation.
    pub fn explicit_on_delete(self) -> Option<ReferentialAction> {
        self.attributes().on_delete.map(|(action, _span)| action)
    }

    /// The onDelete argument on the relation.
    pub fn explicit_on_delete_span(self) -> Option<ast::Span> {
        self.attributes().on_delete.map(|(_action, span)| span)
    }

    /// The onUpdate argument on the relation.
    pub fn explicit_on_update(self) -> Option<ReferentialAction> {
        self.attributes().on_update.map(|(action, _span)| action)
    }

    /// The onUpdate argument on the relation.
    pub fn explicit_on_update_span(self) -> Option<ast::Span> {
        self.attributes().on_update.map(|(_action, span)| span)
    }

    /// The relation name explicitly written in the schema source.
    pub fn explicit_relation_name(self) -> Option<&'db str> {
        self.attributes().name.map(|string_id| &self.db[string_id])
    }

    /// Is there an `@ignore` attribute on the field?
    pub fn is_ignored(self) -> bool {
        self.attributes().is_ignored
    }

    /// Is the field required? (not optional, not list)
    pub fn is_required(self) -> bool {
        self.ast_field().arity.is_required()
    }

    /// The model containing the field.
    pub fn model(self) -> ModelWalker<'db> {
        self.walk(self.attributes().model_id)
    }

    /// A valid relation is defined by two relation fields. This method returns the _other_
    /// relation field in the same relation.
    pub fn opposite_relation_field(self) -> Option<RelationFieldWalker<'db>> {
        self.relation().relation_fields().find(|rf| rf.id != self.id)
    }

    /// The `@relation` attribute in the field AST.
    pub fn relation_attribute(self) -> Option<&'db ast::Attribute> {
        self.attributes().relation_attribute.map(|id| &self.db.ast[id])
    }

    /// Does the relation field reference the passed in model?
    pub fn references_model(self, other: ast::ModelId) -> bool {
        self.attributes().referenced_model == other
    }

    /// The model referenced by the relation.
    pub fn related_model(self) -> ModelWalker<'db> {
        self.db.walk(self.attributes().referenced_model)
    }

    /// The fields in the `@relation(references: ...)` argument.
    pub fn referenced_fields(self) -> Option<impl ExactSizeIterator<Item = ScalarFieldWalker<'db>>> {
        self.attributes()
            .references
            .as_ref()
            .map(|references| references.iter().map(move |field_id| self.walk(*field_id)))
    }

    /// The relation this field is part of.
    pub fn relation(self) -> RelationWalker<'db> {
        self.walk(self.db.relations[self.id])
    }

    /// The name of the relation. Either uses the `name` (or default) argument,
    /// or generates an implicit name.
    pub fn relation_name(self) -> RelationName<'db> {
        self.explicit_relation_name()
            .map(RelationName::Explicit)
            .unwrap_or_else(|| RelationName::generated(self.model().name(), self.related_model().name()))
    }

    /// The arity to enforce, based on the arity of the fields. If any referencing field is
    /// required, this will be required.
    ///
    /// Prisma allows setting the relation field as optional, even if one of the
    /// underlying scalar fields is required. For the purpose of referential
    /// actions, we count the relation field required if any of the underlying
    /// fields is required.
    pub fn referential_arity(self) -> FieldArity {
        let some_required = self
            .fields()
            .map(|mut f| f.any(|f| f.ast_field().arity.is_required()))
            .unwrap_or(false);

        if some_required {
            FieldArity::Required
        } else {
            self.ast_field().arity
        }
    }

    /// Used for validation.
    pub fn references_singular_id_field(self) -> bool {
        let singular_referenced_id = match self.attributes().references.as_deref() {
            Some([field_id]) => field_id,
            Some(_) => return false,
            None => return true, // implicitly, these are referencing the singular id
        };

        matches!(self.related_model().primary_key(), Some(pk) if pk.contains_exactly_fields_by_id(&[*singular_referenced_id]))
    }

    /// The fields in the `fields: [...]` argument in the forward relation field.
    pub fn referencing_fields(self) -> Option<impl ExactSizeIterator<Item = ScalarFieldWalker<'db>> + Clone> {
        self.fields()
    }

    /// The fields in the `fields: [...]` argument in the forward relation field.
    pub fn fields(self) -> Option<impl ExactSizeIterator<Item = ScalarFieldWalker<'db>> + Clone> {
        let attributes = &self.db.types[self.id];
        attributes
            .fields
            .as_ref()
            .map(move |fields| fields.iter().map(move |field_id| self.db.walk(*field_id)))
    }
}

/// The relation name.
#[derive(Debug, Clone)]
pub enum RelationName<'db> {
    /// A relation name specified in the AST.
    Explicit(&'db str),
    /// An inferred relation name.
    Generated(String),
}

impl<'db> PartialEq for RelationName<'db> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Explicit(l0), Self::Explicit(r0)) => l0 == r0,
            (Self::Generated(l0), Self::Generated(r0)) => l0 == r0,
            (Self::Explicit(l0), Self::Generated(r0)) => l0 == r0,
            (Self::Generated(l0), Self::Explicit(r0)) => l0 == r0,
        }
    }
}

impl<'db> Eq for RelationName<'db> {}

impl<'db> PartialOrd for RelationName<'db> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::Explicit(l0), Self::Explicit(r0)) => l0.partial_cmp(r0),
            (Self::Generated(l0), Self::Generated(r0)) => l0.partial_cmp(r0),
            (Self::Explicit(l0), Self::Generated(r0)) => l0.partial_cmp(&r0.as_str()),
            (Self::Generated(l0), Self::Explicit(r0)) => l0.as_str().partial_cmp(*r0),
        }
    }
}

impl<'db> Ord for RelationName<'db> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Explicit(l0), Self::Explicit(r0)) => l0.cmp(r0),
            (Self::Generated(l0), Self::Generated(r0)) => l0.cmp(r0),
            (Self::Explicit(l0), Self::Generated(r0)) => l0.cmp(&r0.as_str()),
            (Self::Generated(l0), Self::Explicit(r0)) => l0.as_str().cmp(*r0),
        }
    }
}

impl<'db> std::hash::Hash for RelationName<'db> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            RelationName::Explicit(s) => s.hash(state),
            RelationName::Generated(s) => s.hash(state),
        }
    }
}

impl<'db> RelationName<'db> {
    pub(crate) fn generated(model_a: &str, model_b: &str) -> Self {
        if model_a < model_b {
            Self::Generated(format!("{model_a}To{model_b}"))
        } else {
            Self::Generated(format!("{model_b}To{model_a}"))
        }
    }
}

impl<'db> fmt::Display for RelationName<'db> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RelationName::Explicit(s) => f.write_str(s),
            RelationName::Generated(s) => f.write_str(s),
        }
    }
}

impl<'db> AsRef<str> for RelationName<'db> {
    fn as_ref(&self) -> &str {
        match self {
            RelationName::Explicit(s) => s,
            RelationName::Generated(s) => s,
        }
    }
}

impl<'db> From<RelationName<'db>> for Cow<'db, str> {
    fn from(name: RelationName<'db>) -> Self {
        match name {
            RelationName::Explicit(name) => Cow::from(name),
            RelationName::Generated(name) => Cow::from(name),
        }
    }
}
