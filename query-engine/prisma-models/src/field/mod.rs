mod composite;
mod relation;
mod scalar;

pub use composite::*;
pub use relation::*;
pub use scalar::*;

// use crate::prelude::*;
use datamodel::ScalarType;
use std::{hash::Hash, sync::Arc};

use crate::ModelRef;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Field {
    Relation(RelationFieldRef),
    Scalar(ScalarFieldRef),
    Composite(CompositeFieldRef),
}

impl Field {
    pub fn name(&self) -> &str {
        match self {
            Field::Scalar(ref sf) => &sf.name,
            Field::Relation(ref rf) => &rf.name,
            Field::Composite(ref cf) => &cf.name,
        }
    }

    pub fn db_name(&self) -> &str {
        match self {
            Field::Scalar(ref sf) => sf.db_name(),
            Field::Relation(ref rf) => &rf.name,
            Field::Composite(ref cf) => cf.db_name(),
        }
    }

    pub fn is_scalar(&self) -> bool {
        matches!(self, Self::Scalar(_))
    }

    pub fn is_relation(&self) -> bool {
        matches!(self, Self::Relation(..))
    }

    pub fn is_composite(&self) -> bool {
        matches!(self, Self::Composite(_))
    }

    pub fn is_id(&self) -> bool {
        match self {
            Field::Scalar(sf) => sf.is_id,
            Field::Relation(_) => false,
            Field::Composite(_) => false,
        }
    }

    pub fn is_list(&self) -> bool {
        match self {
            Field::Scalar(ref sf) => sf.is_list(),
            Field::Relation(ref rf) => rf.is_list(),
            Field::Composite(ref cf) => cf.is_list(),
        }
    }

    pub fn try_into_scalar(self) -> Option<ScalarFieldRef> {
        match self {
            Field::Scalar(scalar) => Some(scalar),
            _ => None,
        }
    }

    pub fn is_required(&self) -> bool {
        match self {
            Field::Scalar(ref sf) => sf.is_required(),
            Field::Relation(ref rf) => rf.is_required(),
            Field::Composite(ref cf) => cf.is_required(),
        }
    }

    pub fn is_unique(&self) -> bool {
        match self {
            Field::Scalar(ref sf) => sf.unique(),
            Field::Relation(_) => false,
            Field::Composite(_) => false,
        }
    }

    pub fn model(&self) -> Option<ModelRef> {
        match self {
            Self::Scalar(sf) => sf.container.as_model(),
            Self::Relation(rf) => Some(rf.model()),
            Self::Composite(cf) => cf.container.as_model(),
        }
    }

    pub fn scalar_fields(&self) -> Vec<ScalarFieldRef> {
        match self {
            Self::Scalar(sf) => vec![sf.clone()],
            Self::Relation(rf) => rf.scalar_fields(),
            Self::Composite(_cf) => vec![], // [Composites] todo
        }
    }

    pub fn downgrade(&self) -> FieldWeak {
        match self {
            Field::Relation(field) => FieldWeak::Relation(Arc::downgrade(field)),
            Field::Scalar(field) => FieldWeak::Scalar(Arc::downgrade(field)),
            Field::Composite(field) => FieldWeak::Composite(Arc::downgrade(field)),
        }
    }

    pub fn as_composite(&self) -> Option<&CompositeFieldRef> {
        if let Self::Composite(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub enum FieldWeak {
    Relation(RelationFieldWeak),
    Scalar(ScalarFieldWeak),
    Composite(CompositeFieldWeak),
}

impl FieldWeak {
    pub fn upgrade(&self) -> Field {
        match self {
            Self::Relation(rf) => rf.upgrade().unwrap().into(),
            Self::Scalar(sf) => sf.upgrade().unwrap().into(),
            Self::Composite(cf) => cf.upgrade().unwrap().into(),
        }
    }
}

impl From<&Field> for FieldWeak {
    fn from(f: &Field) -> Self {
        match f {
            Field::Scalar(sf) => sf.into(),
            Field::Relation(rf) => rf.into(),
            Field::Composite(cf) => cf.into(),
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

impl From<&CompositeFieldRef> for FieldWeak {
    fn from(f: &CompositeFieldRef) -> Self {
        FieldWeak::Composite(Arc::downgrade(f))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[allow(clippy::upper_case_acronyms)]
pub enum TypeIdentifier {
    String,
    Int,
    BigInt,
    Float,
    Decimal,
    Boolean,
    Enum(String),
    UUID,
    Json,
    Xml,
    DateTime,
    Bytes,
    Unsupported,
}

impl TypeIdentifier {
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            TypeIdentifier::Int | TypeIdentifier::BigInt | TypeIdentifier::Float | TypeIdentifier::Decimal
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DateType {
    Date,
    Time,
    DateTime,
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

impl From<CompositeFieldRef> for Field {
    fn from(cf: CompositeFieldRef) -> Self {
        Field::Composite(cf)
    }
}

impl From<ScalarType> for TypeIdentifier {
    fn from(st: ScalarType) -> Self {
        match st {
            ScalarType::String => Self::String,
            ScalarType::Int => Self::Int,
            ScalarType::BigInt => Self::BigInt,
            ScalarType::Float => Self::Float,
            ScalarType::Boolean => Self::Boolean,
            ScalarType::DateTime => Self::DateTime,
            ScalarType::Json => Self::Json,
            ScalarType::Decimal => Self::Decimal,
            ScalarType::Bytes => Self::Bytes,
        }
    }
}
