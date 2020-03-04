mod relation;
mod scalar;

pub use relation::*;
pub use scalar::*;

use crate::prelude::*;
use core::ops::Deref;
use datamodel::ScalarType;
use std::{
    hash::{Hash, Hasher},
    sync::Arc,
};

pub type DataSourceFieldRef = Arc<DataSourceField>;

#[derive(Debug)]
pub enum FieldTemplate {
    Relation(RelationFieldTemplate),
    Scalar(ScalarFieldTemplate),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Field {
    Relation(RelationFieldRef),
    Scalar(ScalarFieldRef),
}

#[derive(Debug, Clone)]
pub enum FieldWeak {
    Relation(RelationFieldWeak),
    Scalar(ScalarFieldWeak),
}

impl FieldWeak {
    pub fn upgrade(&self) -> Field {
        match self {
            Self::Relation(rf) => rf.upgrade().unwrap().into(),
            Self::Scalar(sf) => sf.upgrade().unwrap().into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DataSourceField {
    backing_field: dml::DataSourceField,
    model_field: FieldWeak,
}

impl DataSourceField {
    pub fn new(backing_field: dml::DataSourceField, model_field: FieldWeak) -> Self {
        Self {
            backing_field,
            model_field,
        }
    }

    pub fn model_field(&self) -> Field {
        self.model_field.upgrade()
    }
}

impl Deref for DataSourceField {
    type Target = dml::DataSourceField;

    fn deref(&self) -> &dml::DataSourceField {
        &self.backing_field
    }
}

impl Hash for DataSourceField {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.backing_field.hash(state);
        self.model_field().hash(state);
    }
}

impl Eq for DataSourceField {}

impl PartialEq for DataSourceField {
    fn eq(&self, other: &DataSourceField) -> bool {
        self.name == other.name
    }
}

impl From<&Field> for FieldWeak {
    fn from(f: &Field) -> Self {
        match f {
            Field::Scalar(sf) => sf.into(),
            Field::Relation(rf) => rf.into(),
        }
    }
}

impl From<&ScalarFieldRef> for FieldWeak {
    fn from(f: &ScalarFieldRef) -> Self {
        FieldWeak::Scalar(Arc::downgrade(f))
    }
}

impl From<&RelationFieldRef> for FieldWeak {
    fn from(f: &RelationFieldRef) -> Self {
        FieldWeak::Relation(Arc::downgrade(f))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeIdentifier {
    String,
    Float,
    Boolean,
    Enum(String),
    Json,
    DateTime,
    UUID,
    Int,
}

impl Field {
    pub fn name(&self) -> &str {
        match self {
            Field::Scalar(ref sf) => &sf.name,
            Field::Relation(ref rf) => &rf.name,
        }
    }

    pub fn is_scalar(&self) -> bool {
        match self {
            Field::Scalar(_) => true,
            Field::Relation(_) => false,
        }
    }

    pub fn is_list(&self) -> bool {
        match self {
            Field::Scalar(ref sf) => sf.is_list,
            Field::Relation(ref rf) => rf.is_list,
        }
    }

    pub(crate) fn as_scalar(self) -> Option<ScalarFieldRef> {
        match self {
            Field::Scalar(scalar) => Some(scalar),
            _ => None,
        }
    }

    pub fn is_required(&self) -> bool {
        match self {
            Field::Scalar(ref sf) => sf.is_required,
            Field::Relation(ref rf) => rf.is_required,
        }
    }

    pub fn model(&self) -> ModelRef {
        match self {
            Self::Scalar(sf) => sf.model(),
            Self::Relation(rf) => rf.model(),
        }
    }

    pub fn data_source_fields(&self) -> Vec<DataSourceFieldRef> {
        match self {
            Self::Scalar(sf) => vec![sf.data_source_field().clone()],
            Self::Relation(rf) => rf.data_source_fields().to_vec(),
        }
    }

    pub fn downgrade(&self) -> FieldWeak {
        match self {
            Field::Relation(field) => FieldWeak::Relation(Arc::downgrade(field)),
            Field::Scalar(field) => FieldWeak::Scalar(Arc::downgrade(field)),
        }
    }
}

impl FieldTemplate {
    pub fn build(self, model: ModelWeakRef) -> Field {
        match self {
            FieldTemplate::Scalar(st) => Field::Scalar(st.build(model)),
            FieldTemplate::Relation(rt) => Field::Relation(rt.build(model)),
        }
    }
}

impl From<ScalarFieldRef> for Field {
    fn from(sf: ScalarFieldRef) -> Self {
        Field::Scalar(sf)
    }
}

impl From<RelationFieldRef> for Field {
    fn from(rf: RelationFieldRef) -> Self {
        Field::Relation(rf)
    }
}

impl From<ScalarType> for TypeIdentifier {
    fn from(st: ScalarType) -> Self {
        match st {
            ScalarType::String => Self::String,
            ScalarType::Int => Self::Int,
            ScalarType::Float => Self::Float,
            ScalarType::Boolean => Self::Boolean,
            ScalarType::Decimal => Self::Float,
            ScalarType::DateTime => Self::DateTime,
        }
    }
}
