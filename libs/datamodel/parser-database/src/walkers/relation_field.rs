use crate::{
    ast::{self, FieldArity},
    types::RelationField,
    walkers::{ModelWalker, RelationWalker, ScalarFieldWalker},
    ParserDatabase, ReferentialAction,
};
use std::{
    borrow::Cow,
    fmt,
    hash::{Hash, Hasher},
};

/// A relation field on a model in the schema.
#[derive(Copy, Clone, Debug)]
pub struct RelationFieldWalker<'ast, 'db> {
    pub(crate) model_id: ast::ModelId,
    pub(crate) field_id: ast::FieldId,
    pub(crate) db: &'db ParserDatabase<'ast>,
    pub(crate) relation_field: &'db RelationField<'ast>,
}

impl<'ast, 'db> PartialEq for RelationFieldWalker<'ast, 'db> {
    fn eq(&self, other: &Self) -> bool {
        self.model_id == other.model_id && self.field_id == other.field_id
    }
}

impl<'ast, 'db> Eq for RelationFieldWalker<'ast, 'db> {}

impl<'ast, 'db> Hash for RelationFieldWalker<'ast, 'db> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.model_id.hash(state);
        self.field_id.hash(state);
    }
}

impl<'ast, 'db> RelationFieldWalker<'ast, 'db> {
    /// The ID of the AST node of the field.
    pub fn field_id(self) -> ast::FieldId {
        self.field_id
    }

    /// The foreign key name of the relation (`@relation(map: ...)`).
    pub fn mapped_name(self) -> Option<&'ast str> {
        self.attributes().mapped_name
    }

    /// The field name.
    pub fn name(self) -> &'ast str {
        self.ast_field().name()
    }

    /// The AST node of the field.
    pub fn ast_field(self) -> &'ast ast::Field {
        &self.db.ast[self.model_id][self.field_id]
    }

    pub(crate) fn attributes(self) -> &'db RelationField<'ast> {
        self.relation_field
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
        self.relation_field.name.as_ref().map(|s| self.db.resolve_str(s))
    }

    /// Is there an `@ignore` attribute on the field?
    pub fn is_ignored(self) -> bool {
        self.relation_field.is_ignored
    }

    /// Is the field required? (not optional, not list)
    pub fn is_required(self) -> bool {
        self.ast_field().arity.is_required()
    }

    /// The model containing the field.
    pub fn model(self) -> ModelWalker<'ast, 'db> {
        ModelWalker {
            model_id: self.model_id,
            db: self.db,
            model_attributes: &self.db.types.model_attributes[&self.model_id],
        }
    }

    /// The `@relation` attribute in the field AST.
    pub fn relation_attribute(self) -> Option<&'ast ast::Attribute> {
        self.attributes().relation_attribute
    }

    pub(crate) fn references_model(self, other: ast::ModelId) -> bool {
        self.relation_field.referenced_model == other
    }

    /// The model referenced by the relation.
    pub fn related_model(self) -> ModelWalker<'ast, 'db> {
        let model_id = self.relation_field.referenced_model;

        ModelWalker {
            model_id,
            db: self.db,
            model_attributes: &self.db.types.model_attributes[&model_id],
        }
    }

    /// The fields in the `@relation(references: ...)` argument.
    pub fn referenced_fields(self) -> Option<impl ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>> + 'db> {
        self.attributes().references.as_ref().map(|references| {
            references
                .iter()
                .map(move |field_id| self.related_model().scalar_field(*field_id))
        })
    }

    /// The relation this field is part of.
    pub fn relation(self) -> RelationWalker<'ast, 'db> {
        let model = self.model();
        let mut relations = model.relations_from().chain(model.relations_to());
        relations.find(|r| r.has_field(self.model_id, self.field_id)).unwrap()
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
    pub fn referencing_fields(self) -> Option<impl ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>>> {
        self.fields()
    }

    /// The fields in the `fields: [...]` argument in the forward relation field.
    pub fn fields(self) -> Option<impl ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>>> {
        let model_id = self.model_id;
        let attributes = self.attributes();
        attributes.fields.as_ref().map(move |fields| {
            fields.iter().map(move |field_id| ScalarFieldWalker {
                model_id,
                field_id: *field_id,
                db: self.db,
                scalar_field: &self.db.types.scalar_fields[&(model_id, *field_id)],
            })
        })
    }
}

/// The relation name.
#[derive(Debug, Clone)]
pub enum RelationName<'ast> {
    /// A relation name specified in the AST.
    Explicit(&'ast str),
    /// An inferred relation name.
    Generated(String),
}

impl<'ast> PartialEq for RelationName<'ast> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Explicit(l0), Self::Explicit(r0)) => l0 == r0,
            (Self::Generated(l0), Self::Generated(r0)) => l0 == r0,
            (Self::Explicit(l0), Self::Generated(r0)) => l0 == r0,
            (Self::Generated(l0), Self::Explicit(r0)) => l0 == r0,
        }
    }
}

impl<'ast> Eq for RelationName<'ast> {}

impl<'ast> PartialOrd for RelationName<'ast> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::Explicit(l0), Self::Explicit(r0)) => l0.partial_cmp(r0),
            (Self::Generated(l0), Self::Generated(r0)) => l0.partial_cmp(r0),
            (Self::Explicit(l0), Self::Generated(r0)) => l0.partial_cmp(&r0.as_str()),
            (Self::Generated(l0), Self::Explicit(r0)) => l0.as_str().partial_cmp(*r0),
        }
    }
}

impl<'ast> Ord for RelationName<'ast> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Explicit(l0), Self::Explicit(r0)) => l0.cmp(r0),
            (Self::Generated(l0), Self::Generated(r0)) => l0.cmp(r0),
            (Self::Explicit(l0), Self::Generated(r0)) => l0.cmp(&r0.as_str()),
            (Self::Generated(l0), Self::Explicit(r0)) => l0.as_str().cmp(*r0),
        }
    }
}

impl<'ast> std::hash::Hash for RelationName<'ast> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            RelationName::Explicit(s) => s.hash(state),
            RelationName::Generated(s) => s.hash(state),
        }
    }
}

impl<'ast> RelationName<'ast> {
    pub(crate) fn generated(model_a: &str, model_b: &str) -> Self {
        if model_a < model_b {
            Self::Generated(format!("{}To{}", model_a, model_b))
        } else {
            Self::Generated(format!("{}To{}", model_b, model_a))
        }
    }
}

impl<'ast> fmt::Display for RelationName<'ast> {
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

impl<'ast> From<RelationName<'ast>> for Cow<'ast, str> {
    fn from(name: RelationName<'ast>) -> Self {
        match name {
            RelationName::Explicit(name) => Cow::from(name),
            RelationName::Generated(name) => Cow::from(name),
        }
    }
}
