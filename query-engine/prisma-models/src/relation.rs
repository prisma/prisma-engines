use crate::prelude::*;
use dml::ReferentialAction;
use psl::{
    datamodel_connector::RelationMode,
    parser_database::{walkers, RelationId},
    schema_ast::ast,
};
use std::fmt::Debug;

pub type Relation = crate::Zipper<RelationId>;
pub type RelationRef = Relation;
pub type RelationWeakRef = Relation;

impl Relation {
    pub const MODEL_A_DEFAULT_COLUMN: &'static str = "A";
    pub const MODEL_B_DEFAULT_COLUMN: &'static str = "B";
    pub const TABLE_ALIAS: &'static str = "RelationTable";

    pub fn name(&self) -> String {
        self.walker().relation_name().to_string()
    }

    /// Returns `true` only if the `Relation` is just a link between two
    /// `RelationField`s.
    pub fn is_inline_relation(&self) -> bool {
        self.walker().refine().as_inline().is_some()
    }

    /// Returns `true` if the `Relation` is a table linking two models.
    pub fn is_relation_table(&self) -> bool {
        !self.is_inline_relation()
    }

    /// A model that relates to itself. For example a `Person` that is a parent
    /// can relate to people that are children.
    pub fn is_self_relation(&self) -> bool {
        self.walker().is_self_relation()
    }

    fn sorted_models(&self) -> [walkers::ModelWalker<'_>; 2] {
        let mut models = self.walker().models().map(|id| self.dm.walk(id));
        models.sort_by_key(|m| m.name());
        models
    }

    /// A pointer to the first `Model` in the `Relation`.
    pub fn model_a(&self) -> ModelRef {
        self.dm.find_model_by_id(self.sorted_models()[0].id)
    }

    /// A pointer to the second `Model` in the `Relation`.
    pub fn model_b(&self) -> ModelRef {
        self.dm.find_model_by_id(self.sorted_models()[1].id)
    }

    /// A pointer to the `RelationField` in the first `Model` in the `Relation`.
    pub fn field_a(&self) -> RelationFieldRef {
        self.model_a()
            .fields()
            .find_from_relation(self.id, RelationSide::A)
            .unwrap()
    }

    /// A pointer to the `RelationField` in the second `Model` in the `Relation`.
    pub fn field_b(&self) -> RelationFieldRef {
        self.model_b()
            .fields()
            .find_from_relation(self.id, RelationSide::B)
            .unwrap()
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

    /// Retrieves the onDelete policy for this relation.
    pub fn on_delete(&self) -> ReferentialAction {
        let action = self
            .field_a()
            .on_delete()
            .cloned()
            .or_else(|| self.field_b().on_delete().cloned())
            .unwrap_or(self.field_a().on_delete_default);

        match (action, self.dm.schema.relation_mode()) {
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

        match (action, self.dm.schema.relation_mode()) {
            // NoAction is an alias for Restrict when relationMode = "prisma"
            (ReferentialAction::NoAction, RelationMode::Prisma) => ReferentialAction::Restrict,
            (action, _) => action,
        }
    }

    pub fn manifestation(&self) -> RelationLinkManifestation {
        match self.walker().refine() {
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
            walkers::RefinedRelationWalker::TwoWayEmbeddedManyToMany(_) => {
                RelationLinkManifestation::RelationTable(RelationTable {
                    table: String::new(),
                    model_a_column: "A".into(),
                    model_b_column: "B".into(),
                })
            }
        }
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
