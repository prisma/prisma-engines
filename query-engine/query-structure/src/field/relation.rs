use crate::prelude::*;
use psl::parser_database::{
    ast::FieldArity,
    walkers::{self, RelationFieldId},
};
use std::fmt::Display;

pub type RelationField = crate::Zipper<RelationFieldId>;
pub type RelationFieldRef = RelationField;

impl RelationField {
    pub fn borrowed_name<'a>(&self, schema: &'a psl::ValidatedSchema) -> &'a str {
        schema.db.walk(self.id).name()
    }

    pub fn name(&self) -> &str {
        self.walker().name()
    }

    pub fn arity(&self) -> FieldArity {
        self.walker().ast_field().arity
    }

    pub fn is_list(&self) -> bool {
        matches!(self.arity(), FieldArity::List)
    }

    pub fn is_required(&self) -> bool {
        matches!(self.arity(), FieldArity::Required)
    }

    /// Returns the `FieldSelection` used for this relation fields model.
    ///
    /// ## What is the field selection of a relation field?
    /// The set of fields required by the relation (**on the model of the relation field**) to be able to link the related records.
    ///
    /// In case of a many-to-many relation field, we can make the assumption that the primary identifier of the enclosing model
    /// is the set of linking fields, as this is how Prisma many-to-many works and we only support implicit join tables (i.e. m:n)
    /// in the Prisma style.
    pub fn linking_fields(&self) -> FieldSelection {
        self.linking_fields_impl().into()
    }

    pub fn is_optional(&self) -> bool {
        !self.is_required()
    }

    pub fn model(&self) -> Model {
        self.dm.find_model_by_id(self.walker().model().id)
    }

    pub fn scalar_fields(&self) -> Vec<ScalarFieldRef> {
        self.walker()
            .fields()
            .into_iter()
            .flatten()
            .map(|f| self.map_ref(ScalarFieldId::InModel(f.id)))
            .collect()
    }

    pub fn relation(&self) -> Relation {
        let relation_id = self.dm.walk(self.id).relation().id;
        self.map_ref(relation_id)
    }

    /// Alias for more clarity (in most cases, doesn't add more clarity for self-relations);
    pub fn is_inlined_on_enclosing_model(&self) -> bool {
        self.relation_is_inlined_in_parent()
    }

    /// Inlined in self / model of self
    pub fn relation_is_inlined_in_parent(&self) -> bool {
        match self.walker().relation().refine() {
            walkers::RefinedRelationWalker::Inline(m) => m.forward_relation_field().unwrap().id == self.id,
            _ => false,
        }
    }

    pub fn relation_is_inlined_in_child(&self) -> bool {
        self.relation().is_inline_relation() && !self.relation_is_inlined_in_parent()
    }

    pub fn related_model(&self) -> Model {
        self.dm.find_model_by_id(self.walker().related_model().id)
    }

    pub fn related_field(&self) -> RelationField {
        let id = self.walker().opposite_relation_field().unwrap().id;
        self.map_ref(id)
    }

    pub fn type_identifiers_with_arities(&self) -> Vec<(TypeIdentifier, FieldArity)> {
        self.scalar_fields()
            .iter()
            .map(|f| f.type_identifier_with_arity())
            .collect()
    }

    pub fn referenced_fields(&self) -> Vec<ScalarFieldRef> {
        self.walker()
            .referenced_fields()
            .into_iter()
            .flatten()
            .map(|field| self.map_ref(ScalarFieldId::InModel(field.id)))
            .collect()
    }

    // Scalar fields on the left (source) side of the relation if starting traversal from `self`.
    // Todo This is provisionary.
    pub fn left_scalars(&self) -> Vec<ScalarFieldRef> {
        self.linking_fields_impl()
    }

    pub fn db_names(&self) -> impl Iterator<Item = String> {
        self.scalar_fields().into_iter().map(|f| f.db_name().to_owned())
    }

    fn linking_fields_impl(&self) -> Vec<ScalarFieldRef> {
        let walker = self.walker();
        let relation = walker.relation();

        match relation.refine() {
            walkers::RefinedRelationWalker::Inline(rel) => {
                let forward = rel.forward_relation_field().unwrap();
                if forward.id == self.id {
                    forward
                        .fields()
                        .unwrap()
                        .map(|sf| self.map_ref(ScalarFieldId::InModel(sf.id)))
                        .collect()
                } else {
                    forward
                        .referenced_fields()
                        .unwrap()
                        .map(|sf| self.map_ref(ScalarFieldId::InModel(sf.id)))
                        .collect()
                }
            }
            walkers::RefinedRelationWalker::TwoWayEmbeddedManyToMany(_)
            | walkers::RefinedRelationWalker::ImplicitManyToMany(_) => {
                self.model().primary_identifier().as_scalar_fields().unwrap()
            }
        }
    }
}

impl Display for RelationField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let walker = self.walker();
        write!(f, "{}.{}", walker.model().name(), walker.name())
    }
}

impl std::fmt::Debug for RelationField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("RelationField")
            .field(&format!("{}.{}", self.model().name(), self.name(),))
            .finish()
    }
}
