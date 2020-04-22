mod relation;
mod scalar;

pub use relation::*;
pub use scalar::*;

use crate::prelude::*;
use datamodel::ScalarType;
use std::{hash::Hash, sync::Arc};

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

    pub fn is_id(&self) -> bool {
        match self {
            Field::Scalar(sf) => sf.is_id,
            Field::Relation(rf) => rf.is_id,
        }
    }

    pub fn is_list(&self) -> bool {
        match self {
            Field::Scalar(ref sf) => sf.is_list,
            Field::Relation(ref rf) => rf.is_list,
        }
    }

    pub fn as_scalar(self) -> Option<ScalarFieldRef> {
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

    pub fn is_unique(&self) -> bool {
        match self {
            Field::Scalar(ref sf) => sf.unique(),
            Field::Relation(ref rf) => rf.is_id || rf.is_unique,
        }
    }

    pub fn model(&self) -> ModelRef {
        match self {
            Self::Scalar(sf) => sf.model(),
            Self::Relation(rf) => rf.model(),
        }
    }

    pub fn scalar_fields(&self) -> Vec<ScalarFieldRef> {
        match self {
            Self::Scalar(sf) => vec![sf.clone()],
            Self::Relation(rf) => rf.scalar_fields(),
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
            ScalarType::DateTime => Self::DateTime,
            ScalarType::Json => Self::Json,
        }
    }
}
