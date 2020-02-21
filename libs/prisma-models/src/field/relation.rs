use super::DataSourceField;
use crate::prelude::*;
use datamodel::{FieldArity, RelationInfo};
use once_cell::sync::OnceCell;
use std::{
    hash::{Hash, Hasher},
    sync::{Arc, Weak},
};

/// A short-hand for `Arc<RelationField>`
pub type RelationFieldRef = Arc<RelationField>;
/// A short-hand for `Weak<RelationField>`
pub type RelationFieldWeak = Weak<RelationField>;

#[derive(Debug)]
pub struct RelationFieldTemplate {
    pub name: String,
    pub is_required: bool,
    pub is_list: bool,
    pub is_unique: bool,
    pub is_auto_generated_int_id: bool,
    pub relation_name: String,
    pub relation_side: RelationSide,
    pub data_source_fields: Vec<dml::DataSourceField>,
    pub relation_info: RelationInfo,
}

#[derive(DebugStub, Clone)]
pub struct RelationField {
    pub name: String,
    pub is_required: bool,
    pub is_list: bool,
    pub is_auto_generated_int_id: bool,
    pub relation_name: String,
    pub relation_side: RelationSide,
    pub relation: OnceCell<RelationWeakRef>,
    pub data_source_fields: OnceCell<Vec<DataSourceFieldRef>>,
    pub relation_info: RelationInfo,

    #[debug_stub = "#ModelWeakRef#"]
    pub model: ModelWeakRef,

    pub(crate) is_unique: bool,
}

impl Eq for RelationField {}

impl Hash for RelationField {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.is_required.hash(state);
        self.is_list.hash(state);
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
            && self.is_required == other.is_required
            && self.is_list == other.is_list
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

impl RelationFieldTemplate {
    pub fn build(self, model: ModelWeakRef) -> RelationFieldRef {
        let relation = RelationField {
            name: self.name,
            is_required: self.is_required,
            is_list: self.is_list,
            is_auto_generated_int_id: self.is_auto_generated_int_id,
            is_unique: self.is_unique,
            relation_name: self.relation_name,
            relation_side: self.relation_side,
            model,
            relation: OnceCell::new(),
            data_source_fields: OnceCell::new(),
            relation_info: self.relation_info,
        };

        let arc = Arc::new(relation);
        let fields: Vec<_> = self
            .data_source_fields
            .into_iter()
            .map(|dsf| Arc::new(DataSourceField::new(dsf, FieldWeak::from(&arc))))
            .collect();

        arc.data_source_fields.set(fields).unwrap();
        arc
    }
}

impl RelationField {
    /// Returns the `ModelIdentifier` used for this relation fields model.
    ///
    /// ## What is the model identifier of a relation field?
    /// The set of fields required by the relation (on the model of the relation field) to be able to link the related records.
    ///
    /// In case of a many-to-many relation field, we can make the assumption that the primary identifier of the enclosing model
    /// is the set of linking fields, as this is how Prisma many-to-many works and we only support implicit join tables (i.e. m:n)
    /// in the Prisma style.
    pub fn linking_fields(&self) -> ModelIdentifier {
        if self.relation().is_many_to_many() {
            self.model().primary_identifier()
        } else if self.relation_info.to_fields.is_empty() {
            let related_field = self.related_field();
            let model = self.model();
            let fields = model.fields();

            let to_fields: Vec<_> = related_field
                .relation_info
                .to_fields
                .iter()
                .map(|field_name| {
                    fields
                        .find_from_all(field_name)
                        .expect(&format!(
                            "Invalid data model: To field {} can't be resolved on model {}",
                            field_name, model.name
                        ))
                        .clone()
                })
                .collect();

            ModelIdentifier::new(to_fields)
        } else {
            ModelIdentifier::new(vec![Arc::new(self.clone()).into()])
        }
    }

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

    /// Alias for more clarity. [DTODO] This is actually incorrect in self-relation cases...
    pub fn is_inlined_on_enclosing_model(&self) -> bool {
        self.relation_is_inlined_in_parent()
    }

    /// Inlined in self / model of self
    pub fn relation_is_inlined_in_parent(&self) -> bool {
        let relation = self.relation();

        match relation.manifestation {
            RelationLinkManifestation::Inline(ref m) => {
                let is_self_rel = relation.is_self_relation();

                if is_self_rel {
                    !self.relation_info.to_fields.is_empty()
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

    pub fn data_source_fields(&self) -> &[DataSourceFieldRef] {
        self.data_source_fields
            .get()
            .ok_or_else(|| String::from("Data source fields must be set!"))
            .unwrap()
    }

    pub fn type_identifiers_with_arities(&self) -> Vec<(TypeIdentifier, FieldArity)> {
        self.data_source_fields()
            .iter()
            .map(|dsf| (dsf.field_type.into(), dsf.arity))
            .collect()
    }

    pub fn db_names(&self) -> impl Iterator<Item = &str> {
        self.data_source_fields().into_iter().map(|dsf| dsf.name.as_str())
    }
}
