use crate::prelude::*;
use datamodel::{DataSourceField, FieldArity};
use once_cell::sync::OnceCell;
use std::{
    hash::{Hash, Hasher},
    sync::{Arc, Weak},
};

pub type RelationFieldRef = Arc<RelationField>;
pub type RelationFieldWeak = Weak<RelationField>;

#[derive(Debug)]
pub struct RelationFieldTemplate {
    pub name: String,
    pub type_identifier: TypeIdentifier,
    pub is_required: bool,
    pub is_list: bool,
    pub is_unique: bool,
    pub is_hidden: bool,
    pub is_auto_generated_int_id: bool,
    pub relation_name: String,
    pub relation_side: RelationSide,
    pub data_source_fields: Vec<DataSourceField>,
}

#[derive(DebugStub)]
pub struct RelationField {
    pub name: String,
    pub type_identifier: TypeIdentifier,
    pub is_required: bool,
    pub is_list: bool,
    pub is_hidden: bool,
    pub is_auto_generated_int_id: bool,
    pub relation_name: String,
    pub relation_side: RelationSide,
    pub relation: OnceCell<RelationWeakRef>,
    pub data_source_fields: Vec<DataSourceField>,

    #[debug_stub = "#ModelWeakRef#"]
    pub model: ModelWeakRef,

    pub(crate) is_unique: bool,
}

impl Eq for RelationField {}

impl Hash for RelationField {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.type_identifier.hash(state);
        self.is_required.hash(state);
        self.is_list.hash(state);
        self.is_hidden.hash(state);
        self.is_auto_generated_int_id.hash(state);
        self.relation_name.hash(state);
        self.relation_side.hash(state);
        self.is_unique.hash(state);
        self.model().hash(state);
    }
}

impl PartialEq for RelationField {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.type_identifier == other.type_identifier
            && self.is_required == other.is_required
            && self.is_list == other.is_list
            && self.is_hidden == other.is_hidden
            && self.is_auto_generated_int_id == other.is_auto_generated_int_id
            && self.relation_name == other.relation_name
            && self.relation_side == other.relation_side
            && self.is_unique == other.is_unique
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

impl RelationField {
    pub fn is_optional(&self) -> bool {
        !self.is_required
    }

    pub fn is_unique(&self) -> bool {
        self.is_unique
    }

    pub fn model(&self) -> ModelRef {
        self.model
            .upgrade()
            .expect("Model does not exist anymore. Parent model got deleted without deleting the child.")
    }

    pub fn relation(&self) -> RelationRef {
        self.relation
            .get_or_init(|| {
                self.model()
                    .internal_data_model()
                    .find_relation(&self.relation_name)
                    .unwrap()
            })
            .upgrade()
            .unwrap()
    }

    /// Alias for more clarity.
    pub fn is_inlined_in_enclosing_model(&self) -> bool {
        self.relation_is_inlined_in_parent()
    }

    /// Inlined in self / model of self
    pub fn relation_is_inlined_in_parent(&self) -> bool {
        let relation = self.relation();

        match relation.manifestation {
            RelationLinkManifestation::Inline(ref m) => {
                let is_self_rel = relation.is_self_relation();

                if is_self_rel && self.is_hidden {
                    false
                } else if is_self_rel && (self.relation_side == RelationSide::B || self.related_field().is_hidden) {
                    true
                } else if is_self_rel && self.relation_side == RelationSide::A {
                    false
                } else {
                    m.in_table_of_model_name == self.model().name
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
        self.relation().name == relation_name && self.relation_side == side
    }

    pub fn type_identifier_with_arity(&self) -> (TypeIdentifier, FieldArity) {
        let arity = match (self.is_list, self.is_required) {
            (true, _) => FieldArity::List,
            (false, true) => FieldArity::Required,
            (false, false) => FieldArity::Optional,
        };

        (self.type_identifier, arity)
    }
}
