use std::{
    borrow::Cow,
    fmt,
    hash::{Hash, Hasher},
};

use crate::{
    ast::{self, FieldArity},
    common::constraint_names::ConstraintNames,
    transform::ast_to_dml::db::{types::RelationField, ParserDatabase},
};

use super::{ModelWalker, ScalarFieldWalker};

#[derive(Copy, Clone)]
pub(crate) struct RelationFieldWalker<'ast, 'db> {
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
    pub(crate) fn field_id(self) -> ast::FieldId {
        self.field_id
    }

    pub(crate) fn name(self) -> &'ast str {
        self.ast_field().name()
    }

    pub(crate) fn ast_field(self) -> &'ast ast::Field {
        &self.db.ast[self.model_id][self.field_id]
    }

    pub(crate) fn attributes(self) -> &'db RelationField<'ast> {
        self.relation_field
    }

    /// The relation name explicitly written in the schema source.
    pub(crate) fn explicit_relation_name(self) -> Option<&'ast str> {
        self.relation_field.name
    }

    pub(crate) fn model(self) -> ModelWalker<'ast, 'db> {
        ModelWalker {
            model_id: self.model_id,
            db: self.db,
            model_attributes: &self.db.types.model_attributes[&self.model_id],
        }
    }

    pub(crate) fn references_model(self, other: ast::ModelId) -> bool {
        self.relation_field.referenced_model == other
    }

    pub(crate) fn related_model(self) -> ModelWalker<'ast, 'db> {
        let model_id = self.relation_field.referenced_model;

        ModelWalker {
            model_id,
            db: self.db,
            model_attributes: &self.db.types.model_attributes[&model_id],
        }
    }

    pub(crate) fn referenced_fields(self) -> Option<impl ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>> + 'db> {
        self.attributes().references.as_ref().map(|references| {
            references
                .iter()
                .map(move |field_id| self.related_model().scalar_field(*field_id))
        })
    }

    /// This will be None for virtual relation fields (when no `fields` argument is passed).
    pub(crate) fn final_foreign_key_name(self) -> Option<Cow<'ast, str>> {
        self.attributes().fk_name.map(Cow::Borrowed).or_else(|| {
            let fields = self.relation_field.fields.as_ref()?;
            let model = self.db.walk_model(self.model_id);
            let table_name = model.final_database_name();
            let column_names: Vec<&str> = model.get_field_db_names(fields).collect();

            Some(
                ConstraintNames::foreign_key_constraint_name(table_name, &column_names, self.db.active_connector())
                    .into(),
            )
        })
    }

    /// The name of the relation. Either uses the `name` (or default) argument,
    /// or generates an implicit name.
    pub(crate) fn relation_name(self) -> RelationName<'ast> {
        self.explicit_relation_name()
            .map(RelationName::Explicit)
            .unwrap_or_else(|| RelationName::generated(self.model().name(), self.related_model().name()))
    }

    pub(crate) fn referential_arity(self) -> FieldArity {
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
    pub(crate) fn references_singular_id_field(self) -> bool {
        let singular_referenced_id = match self.attributes().references.as_deref() {
            Some([field_id]) => field_id,
            Some(_) => return false,
            None => return true, // implicitly, these are referencing the singular id
        };

        matches!(self.related_model().primary_key(), Some(pk) if pk.contains_exactly_fields_by_id(&[*singular_referenced_id]))
    }

    pub(crate) fn fields(self) -> Option<impl ExactSizeIterator<Item = ScalarFieldWalker<'ast, 'db>>> {
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

#[derive(Debug, Clone)]
pub(crate) enum RelationName<'ast> {
    Explicit(&'ast str),
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

impl<'ast> AsRef<str> for RelationName<'ast> {
    fn as_ref(&self) -> &str {
        match self {
            RelationName::Explicit(s) => s,
            RelationName::Generated(s) => s,
        }
    }
}
