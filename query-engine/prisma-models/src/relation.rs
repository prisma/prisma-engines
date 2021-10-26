use crate::prelude::*;
use datamodel::ReferentialAction;
use once_cell::sync::OnceCell;
use std::{
    fmt::Debug,
    sync::{Arc, Weak},
};

pub type RelationRef = Arc<Relation>;
pub type RelationWeakRef = Weak<Relation>;

/// A relation between two models. Can be either using a `RelationTable` or
/// model a direct link between two `RelationField`s.
pub struct Relation {
    pub name: String,

    pub(crate) model_a_name: String,
    pub(crate) model_b_name: String,

    pub(crate) model_a: OnceCell<ModelWeakRef>,
    pub(crate) model_b: OnceCell<ModelWeakRef>,

    pub(crate) field_a: OnceCell<Weak<RelationField>>,
    pub(crate) field_b: OnceCell<Weak<RelationField>>,

    pub manifestation: RelationLinkManifestation,
    pub internal_data_model: InternalDataModelWeakRef,
}

impl Relation {
    pub const MODEL_A_DEFAULT_COLUMN: &'static str = "A";
    pub const MODEL_B_DEFAULT_COLUMN: &'static str = "B";
    pub const TABLE_ALIAS: &'static str = "RelationTable";

    /// Returns `true` only if the `Relation` is just a link between two
    /// `RelationField`s.
    pub fn is_inline_relation(&self) -> bool {
        matches!(self.manifestation, RelationLinkManifestation::Inline(_))
    }

    /// Returns `true` if the `Relation` is a table linking two models.
    pub fn is_relation_table(&self) -> bool {
        !self.is_inline_relation()
    }

    /// A model that relates to itself. For example a `Person` that is a parent
    /// can relate to people that are children.
    pub fn is_self_relation(&self) -> bool {
        self.model_a_name == self.model_b_name
    }

    /// A pointer to the first `Model` in the `Relation`.
    pub fn model_a(&self) -> ModelRef {
        self.model_a
            .get_or_init(|| {
                let model = self.internal_data_model().find_model(&self.model_a_name).unwrap();
                Arc::downgrade(&model)
            })
            .upgrade()
            .expect("Model A deleted without deleting the relations in internal_data_model.")
    }

    /// A pointer to the second `Model` in the `Relation`.
    pub fn model_b(&self) -> ModelRef {
        self.model_b
            .get_or_init(|| {
                let model = self.internal_data_model().find_model(&self.model_b_name).unwrap();
                Arc::downgrade(&model)
            })
            .upgrade()
            .expect("Model B deleted without deleting the relations in internal_data_model.")
    }

    /// A pointer to the `RelationField` in the first `Model` in the `Relation`.
    pub fn field_a(&self) -> RelationFieldRef {
        self.field_a
            .get_or_init(|| {
                let field = self
                    .model_a()
                    .fields()
                    .find_from_relation(&self.name, RelationSide::A)
                    .unwrap();

                Arc::downgrade(&field)
            })
            .upgrade()
            .expect("Field A deleted without deleting the relations in internal_data_model.")
    }

    /// A pointer to the `RelationField` in the second `Model` in the `Relation`.
    pub fn field_b(&self) -> RelationFieldRef {
        self.field_b
            .get_or_init(|| {
                let field = self
                    .model_b()
                    .fields()
                    .find_from_relation(&self.name, RelationSide::B)
                    .unwrap();

                Arc::downgrade(&field)
            })
            .upgrade()
            .expect("Field B deleted without deleting the relations in internal_data_model.")
    }

    /// Practically deprecated with Prisma 2.
    pub fn is_many_to_many(&self) -> bool {
        self.field_a().is_list() && self.field_b().is_list()
    }

    pub fn is_one_to_one(&self) -> bool {
        !self.field_a().is_list() && !self.field_b().is_list()
    }

    pub fn is_one_to_many(&self) -> bool {
        !self.is_many_to_many() && !self.is_one_to_one()
    }

    pub fn internal_data_model(&self) -> InternalDataModelRef {
        self.internal_data_model
            .upgrade()
            .expect("InternalDataModel does not exist anymore. Parent internal_data_model is deleted without deleting the child internal_data_model.")
    }

    /// Retrieves the onDelete policy for this relation.
    pub fn on_delete(&self) -> ReferentialAction {
        self.field_a()
            .on_delete()
            .cloned()
            .or_else(|| self.field_b().on_delete().cloned())
            .unwrap_or(self.field_a().on_delete_default)
    }
}

impl Debug for Relation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Relation")
            .field("name", &self.name)
            .field("model_a_name", &self.model_a_name)
            .field("model_b_name", &self.model_b_name)
            .field("model_a", &self.model_a)
            .field("model_b", &self.model_b)
            .field("field_a", &self.field_a)
            .field("field_b", &self.field_b)
            .field("manifestation", &self.manifestation)
            .field("internal_data_model", &"#InternalDataModelWeakRef#")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RelationLinkManifestation {
    Inline(InlineRelation),
    RelationTable(RelationTable),
}

#[derive(Debug, Clone, PartialEq)]
pub struct InlineRelation {
    pub in_table_of_model_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RelationTable {
    pub table: String,
    pub model_a_column: String,
    pub model_b_column: String,
}
