use crate::prelude::*;
use dml::ReferentialAction;
use once_cell::sync::OnceCell;
use psl::{datamodel_connector::RelationMode, parser_database::walkers, schema_ast::ast};
use std::{
    fmt::Debug,
    sync::{Arc, Weak},
};

pub type RelationRef = Arc<Relation>;
pub type RelationWeakRef = Weak<Relation>;

/// A relation between two models. Can be either using a `RelationTable` or
/// model a direct link between two `RelationField`s.
pub struct Relation {
    pub id: psl::parser_database::RelationId,

    pub(crate) model_a_id: ast::ModelId,
    pub(crate) model_b_id: ast::ModelId,

    pub(crate) model_a: OnceCell<ModelWeakRef>,
    pub(crate) model_b: OnceCell<ModelWeakRef>,

    pub(crate) field_a: OnceCell<Weak<RelationField>>,
    pub(crate) field_b: OnceCell<Weak<RelationField>>,

    pub(crate) internal_data_model: InternalDataModelWeakRef,
}

impl Relation {
    pub const MODEL_A_DEFAULT_COLUMN: &'static str = "A";
    pub const MODEL_B_DEFAULT_COLUMN: &'static str = "B";
    pub const TABLE_ALIAS: &'static str = "RelationTable";

    pub fn zipper(&self) -> crate::RelationZipper {
        self.internal_data_model().zip(self.id)
    }

    pub fn name(&self) -> String {
        self.zipper().walker().relation_name().to_string()
    }

    /// Returns `true` only if the `Relation` is just a link between two
    /// `RelationField`s.
    pub fn is_inline_relation(&self) -> bool {
        self.zipper().walker().refine().as_inline().is_some()
    }

    /// Returns `true` if the `Relation` is a table linking two models.
    pub fn is_relation_table(&self) -> bool {
        !self.is_inline_relation()
    }

    /// A model that relates to itself. For example a `Person` that is a parent
    /// can relate to people that are children.
    pub fn is_self_relation(&self) -> bool {
        self.model_a_id == self.model_b_id
    }

    /// A pointer to the first `Model` in the `Relation`.
    pub fn model_a(&self) -> ModelRef {
        self.model_a
            .get_or_init(|| {
                let model = self.internal_data_model().find_model_by_id(self.model_a_id);
                Arc::downgrade(&model)
            })
            .upgrade()
            .expect("Model A deleted without deleting the relations in internal_data_model.")
    }

    /// A pointer to the second `Model` in the `Relation`.
    pub fn model_b(&self) -> ModelRef {
        self.model_b
            .get_or_init(|| {
                let model = self.internal_data_model().find_model_by_id(self.model_b_id);
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
                    .find_from_relation(&self.name(), RelationSide::A)
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
                    .find_from_relation(&self.name(), RelationSide::B)
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
        let action = self
            .field_a()
            .on_delete()
            .cloned()
            .or_else(|| self.field_b().on_delete().cloned())
            .unwrap_or(self.field_a().on_delete_default);

        match (action, self.internal_data_model().schema.relation_mode()) {
            // NoAction is an alias for Restrict when relationMode = "prisma"
            (ReferentialAction::NoAction, RelationMode::Prisma) => ReferentialAction::Restrict,
            (action, _) => action,
        }
    }

    /// Retrieves the onUpdate policy for this relation.
    pub fn on_update(&self) -> ReferentialAction {
        let action = self
            .field_a()
            .on_update()
            .cloned()
            .or_else(|| self.field_b().on_update().cloned())
            .unwrap_or(self.field_a().on_update_default);

        match (action, self.internal_data_model().schema.relation_mode()) {
            // NoAction is an alias for Restrict when relationMode = "prisma"
            (ReferentialAction::NoAction, RelationMode::Prisma) => ReferentialAction::Restrict,
            (action, _) => action,
        }
    }

    pub fn manifestation(&self) -> RelationLinkManifestation {
        match self.zipper().walker().refine() {
            walkers::RefinedRelationWalker::Inline(rel) => RelationLinkManifestation::Inline(InlineRelation {
                in_table_of_model: rel.referencing_model().id,
            }),
            walkers::RefinedRelationWalker::ImplicitManyToMany(rel) => {
                RelationLinkManifestation::RelationTable(RelationTable {
                    table: format!("_{}", rel.relation_name()),
                    model_a_column: "A".into(),
                    model_b_column: "B".into(),
                })
            }
            walkers::RefinedRelationWalker::TwoWayEmbeddedManyToMany(_) => todo!(),
        }
    }
}

impl Debug for Relation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Relation")
            .field("model_a_name", &self.model_a_id)
            .field("model_b_name", &self.model_b_id)
            .field("model_a", &self.model_a)
            .field("model_b", &self.model_b)
            .field("field_a", &self.field_a)
            .field("field_b", &self.field_b)
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
    pub in_table_of_model: ast::ModelId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RelationTable {
    pub table: String,
    pub model_a_column: String,
    pub model_b_column: String,
}
