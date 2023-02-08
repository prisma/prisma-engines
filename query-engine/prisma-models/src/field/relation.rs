use crate::prelude::*;
use dml::{FieldArity, ReferentialAction, RelationInfo};
use once_cell::sync::OnceCell;
use std::{
    fmt::{Debug, Display},
    hash::{Hash, Hasher},
    sync::{Arc, Weak},
};

pub type RelationFieldRef = Arc<RelationField>;
pub type RelationFieldWeak = Weak<RelationField>;

#[derive(Clone)]
pub struct RelationField {
    pub(crate) name: String,
    pub arity: FieldArity,
    pub relation_name: String,
    pub relation_side: RelationSide,
    pub relation: OnceCell<RelationWeakRef>,
    pub(crate) relation_info: RelationInfo,

    pub on_delete_default: ReferentialAction,
    pub on_update_default: ReferentialAction,

    pub(crate) model: ModelWeakRef,
    pub(crate) fields: OnceCell<Vec<ScalarFieldWeak>>,
}

impl RelationField {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_list(&self) -> bool {
        matches!(self.arity, FieldArity::List)
    }

    pub fn is_required(&self) -> bool {
        matches!(self.arity, FieldArity::Required)
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
        if self.relation().is_many_to_many() {
            self.model().primary_identifier()
        } else if self.relation_info.references.is_empty() {
            let related_field = self.related_field();
            let model = self.model();
            let fields = model.fields();

            let referenced_fields: Vec<_> = related_field
                .relation_info
                .references
                .iter()
                .map(|field_name| {
                    fields
                        .find_from_all(field_name)
                        .unwrap_or_else(|_| {
                            panic!(
                                "Invalid data model: To field {} can't be resolved on model {}",
                                field_name, model.name
                            )
                        })
                        .clone()
                })
                .collect();

            FieldSelection::from(referenced_fields)
        } else {
            FieldSelection::from(self)
        }
    }

    pub fn is_optional(&self) -> bool {
        !self.is_required()
    }

    pub fn model(&self) -> ModelRef {
        self.model
            .upgrade()
            .expect("Model does not exist anymore. Parent model got deleted without deleting the child.")
    }

    pub fn scalar_fields(&self) -> Vec<ScalarFieldRef> {
        let fields = self.fields.get_or_init(|| {
            let model = self.model();
            let fields = model.fields();

            self.relation_info
                .fields
                .iter()
                .map(|f| {
                    Arc::downgrade(&fields.find_from_scalar(f).unwrap_or_else(|_| {
                        panic!(
                            "Expected '{}' to be a scalar field on model '{}', found none.",
                            f, model.name
                        )
                    }))
                })
                .collect()
        });

        fields.iter().map(|f| f.upgrade().unwrap()).collect()
    }

    pub fn relation(&self) -> RelationRef {
        self.relation
            .get_or_init(|| {
                self.model()
                    .internal_data_model()
                    .find_relation(
                        (self.model().id, self.relation_info.referenced_model),
                        &self.relation_name,
                    )
                    .unwrap()
            })
            .upgrade()
            .unwrap()
    }

    /// Alias for more clarity (in most cases, doesn't add more clarity for self-relations);
    pub fn is_inlined_on_enclosing_model(&self) -> bool {
        self.relation_is_inlined_in_parent()
    }

    /// Inlined in self / model of self
    pub fn relation_is_inlined_in_parent(&self) -> bool {
        let relation = &self.relation();

        match relation.manifestation() {
            RelationLinkManifestation::Inline(ref m) => {
                let is_self_rel = relation.is_self_relation();

                if is_self_rel {
                    !self.relation_info.references.is_empty()
                } else {
                    m.in_table_of_model == self.model().id
                }
            }
            _ => false,
        }
    }

    pub fn relation_is_inlined_in_child(&self) -> bool {
        self.relation().is_inline_relation() && !self.relation_is_inlined_in_parent()
    }

    pub fn related_model(&self) -> ModelRef {
        match self.relation_side {
            RelationSide::A => self.relation().model_b(),
            RelationSide::B => self.relation().model_a(),
        }
    }

    pub fn related_field(&self) -> Arc<RelationField> {
        match self.relation_side {
            RelationSide::A => self.relation().field_b(),
            RelationSide::B => self.relation().field_a(),
        }
    }

    pub fn is_relation_with_name_and_side(&self, relation_name: &str, side: RelationSide) -> bool {
        self.relation().name() == relation_name && self.relation_side == side
    }

    pub fn type_identifiers_with_arities(&self) -> Vec<(TypeIdentifier, FieldArity)> {
        self.scalar_fields()
            .iter()
            .map(|f| f.type_identifier_with_arity())
            .collect()
    }

    pub fn referenced_fields(&self) -> Vec<ScalarFieldRef> {
        self.relation_info
            .references
            .iter()
            .map(|field_name| self.related_model().fields().find_from_scalar(field_name).unwrap())
            .collect()
    }

    // Scalar fields on the left (source) side of the relation if starting traversal from `self`.
    // Todo This is provisionary.
    pub fn left_scalars(&self) -> Vec<ScalarFieldRef> {
        if self.relation().is_many_to_many() {
            self.model()
                .primary_identifier()
                .as_scalar_fields()
                .expect("Left scalars contain non-scalar selections.")
        } else if self.is_inlined_on_enclosing_model() {
            self.scalar_fields()
        } else {
            self.related_field().referenced_fields()
        }
    }

    // Scalar fields on the right (target) side of the relation if starting traversal from `self`.
    // Todo This is provisionary.
    pub fn right_scalars(&self) -> Vec<ScalarFieldRef> {
        if self.relation().is_many_to_many() {
            let related_field = self.related_field();
            let related_model = self.related_model();

            if related_field.relation_info.fields.is_empty() {
                related_model
                    .primary_identifier()
                    .as_scalar_fields()
                    .expect("Right scalars contain non-scalar selections.")
            } else {
                related_field
                    .relation_info
                    .fields
                    .iter()
                    .map(|f| related_model.fields().find_from_scalar(f.as_str()).unwrap())
                    .collect()
            }
        } else if self.is_inlined_on_enclosing_model() {
            self.referenced_fields()
        } else {
            self.related_field().scalar_fields()
        }
    }

    pub fn db_names(&self) -> impl Iterator<Item = String> {
        self.scalar_fields().into_iter().map(|f| f.db_name().to_owned())
    }

    pub fn on_delete(&self) -> Option<&ReferentialAction> {
        self.relation_info.on_delete.as_ref()
    }

    pub fn on_update(&self) -> Option<&ReferentialAction> {
        self.relation_info.on_update.as_ref()
    }

    pub fn relation_info(&self) -> &RelationInfo {
        &self.relation_info
    }
}

impl Debug for RelationField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RelationField")
            .field("name", &self.name)
            .field("arity", &self.arity)
            .field("relation_name", &self.relation_name)
            .field("relation_side", &self.relation_side)
            .field("relation", &self.relation)
            .field("relation_info", &self.relation_info)
            .field("model", &"#ModelWeakRef#")
            .field("fields", &self.fields)
            .finish()
    }
}

impl Display for RelationField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.model().name, self.name)
    }
}

impl Eq for RelationField {}

impl Hash for RelationField {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.arity.hash(state);
        self.relation_name.hash(state);
        self.relation_side.hash(state);
        self.model().hash(state);
    }
}

impl PartialEq for RelationField {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.arity == other.arity
            && self.relation_name == other.relation_name
            && self.relation_side == other.relation_side
            && self.model() == other.model()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RelationSide {
    A,
    B,
}

impl RelationSide {
    pub fn opposite(self) -> RelationSide {
        match self {
            RelationSide::A => RelationSide::B,
            RelationSide::B => RelationSide::A,
        }
    }

    pub fn is_a(self) -> bool {
        self == RelationSide::A
    }

    pub fn is_b(self) -> bool {
        self == RelationSide::B
    }
}
