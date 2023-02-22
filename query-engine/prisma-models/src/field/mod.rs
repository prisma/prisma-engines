mod composite;
mod relation;
mod scalar;

pub use composite::*;
pub use relation::*;
pub use scalar::*;

use crate::{ast, ModelRef};
use psl::parser_database::{walkers, ScalarType};
use std::{borrow::Cow, hash::Hash};

pub type FieldWeak = Field;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Field {
    Relation(RelationFieldRef),
    Scalar(ScalarFieldRef),
    Composite(CompositeFieldRef),
}

impl Field {
    pub fn name(&self) -> &str {
        match self {
            Field::Scalar(ref sf) => sf.name(),
            Field::Relation(ref rf) => rf.walker().name(),
            Field::Composite(ref cf) => cf.name(),
        }
    }

    pub fn db_name(&self) -> &str {
        match self {
            Field::Scalar(ref sf) => sf.db_name(),
            Field::Relation(rf) => rf.name(),
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

    pub fn into_scalar(self) -> Option<ScalarFieldRef> {
        match self {
            Field::Scalar(sf) => Some(sf),
            _ => None,
        }
    }

    pub fn is_id(&self) -> bool {
        match self {
            Field::Scalar(sf) => sf.is_id(),
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
            Self::Scalar(sf) => sf.container().as_model(),
            Self::Relation(rf) => Some(rf.model()),
            Self::Composite(cf) => cf.container().as_model(),
        }
    }

    pub fn scalar_fields(&self) -> Vec<ScalarFieldRef> {
        match self {
            Self::Scalar(sf) => vec![sf.clone()],
            Self::Relation(rf) => rf.scalar_fields(),
            Self::Composite(_cf) => vec![], // [Composites] todo
        }
    }

    pub fn as_composite(&self) -> Option<&CompositeFieldRef> {
        if let Self::Composite(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_scalar(&self) -> Option<&ScalarFieldRef> {
        if let Self::Scalar(v) = self {
            Some(v)
        } else {
            None
        }
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
    Enum(ast::EnumId),
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

    pub fn type_name(&self, schema: &psl::ValidatedSchema) -> Cow<'static, str> {
        match self {
            TypeIdentifier::String => "String".into(),
            TypeIdentifier::Int => "Int".into(),
            TypeIdentifier::BigInt => "BigInt".into(),
            TypeIdentifier::Float => "Float".into(),
            TypeIdentifier::Decimal => "Decimal".into(),
            TypeIdentifier::Boolean => "Bool".into(),
            TypeIdentifier::Enum(enum_id) => {
                let enum_name = schema.db.walk(*enum_id).name();
                format!("Enum{enum_name}").into()
            }
            TypeIdentifier::UUID => "UUID".into(),
            TypeIdentifier::Json => "Json".into(),
            TypeIdentifier::Xml => "Xml".into(),
            TypeIdentifier::DateTime => "DateTime".into(),
            TypeIdentifier::Bytes => "Bytes".into(),
            TypeIdentifier::Unsupported => "Unsupported".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DateType {
    Date,
    Time,
    DateTime,
}

impl From<(crate::InternalDataModelRef, walkers::CompositeTypeFieldWalker<'_>)> for Field {
    fn from((dm, sf): (crate::InternalDataModelRef, walkers::CompositeTypeFieldWalker<'_>)) -> Self {
        if sf.r#type().as_composite_type().is_some() {
            Field::Composite(dm.zip(CompositeFieldId::InCompositeType(sf.id)))
        } else {
            Field::Scalar(dm.zip(ScalarFieldId::InCompositeType(sf.id)))
        }
    }
}

impl From<(crate::InternalDataModelRef, walkers::ScalarFieldWalker<'_>)> for Field {
    fn from((dm, sf): (crate::InternalDataModelRef, walkers::ScalarFieldWalker<'_>)) -> Self {
        if sf.scalar_field_type().as_composite_type().is_some() {
            Field::Composite(dm.zip(CompositeFieldId::InModel(sf.id)))
        } else {
            Field::Scalar(dm.zip(ScalarFieldId::InModel(sf.id)))
        }
    }
}

impl From<(crate::InternalDataModelRef, walkers::RelationFieldWalker<'_>)> for Field {
    fn from((dm, rf): (crate::InternalDataModelRef, walkers::RelationFieldWalker<'_>)) -> Self {
        Field::Relation(dm.zip(rf.id))
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
