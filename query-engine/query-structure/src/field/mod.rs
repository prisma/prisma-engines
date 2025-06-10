mod composite;
mod relation;
mod scalar;

pub use composite::*;
use prisma_value::PrismaValueType;
pub use relation::*;
pub use scalar::*;

use crate::{parent_container::ParentContainer, Model, NativeTypeInstance, Zipper};
use psl::{
    parser_database::{walkers, EnumId, ScalarType},
    schema_ast::ast::FieldArity,
};
use std::{borrow::Cow, hash::Hash};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Field {
    Relation(RelationFieldRef),
    Scalar(ScalarFieldRef),
    Composite(CompositeFieldRef),
}

impl Field {
    pub fn borrowed_name<'a>(&self, schema: &'a psl::ValidatedSchema) -> &'a str {
        match self {
            Field::Relation(rf) => schema.db.walk(rf.id).name(),
            Field::Scalar(sf) => sf.borrowed_name(schema),
            Field::Composite(cf) => cf.borrowed_name(schema),
        }
    }

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

    pub fn model(&self) -> Option<Model> {
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

    pub fn related_container(&self) -> ParentContainer {
        match self {
            Field::Relation(rf) => ParentContainer::from(rf.related_model()),
            Field::Scalar(sf) => sf.container(),
            Field::Composite(cf) => ParentContainer::from(cf.typ()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Copy)]
#[allow(clippy::upper_case_acronyms)]
pub enum TypeIdentifier {
    String,
    Int,
    BigInt,
    Float,
    Decimal,
    Boolean,
    Enum(EnumId),
    UUID,
    Json,
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

    /// Returns `true` if the type identifier is [`Enum`].
    pub fn is_enum(&self) -> bool {
        matches!(self, Self::Enum(..))
    }

    /// Returns `true` if the type identifier is [`Json`].
    pub fn is_json(&self) -> bool {
        matches!(self, Self::Json)
    }
}

pub type Type = Zipper<TypeIdentifier>;

impl Type {
    pub fn type_name(&self) -> Cow<'static, str> {
        match self.id {
            TypeIdentifier::String => "String".into(),
            TypeIdentifier::Int => "Int".into(),
            TypeIdentifier::BigInt => "BigInt".into(),
            TypeIdentifier::Float => "Float".into(),
            TypeIdentifier::Decimal => "Decimal".into(),
            TypeIdentifier::Boolean => "Bool".into(),
            TypeIdentifier::Enum(enum_id) => {
                let enum_name = self.dm.walk(enum_id).name();
                format!("Enum{enum_name}").into()
            }
            TypeIdentifier::UUID => "UUID".into(),
            TypeIdentifier::Json => "Json".into(),
            TypeIdentifier::DateTime => "DateTime".into(),
            TypeIdentifier::Bytes => "Bytes".into(),
            TypeIdentifier::Unsupported => "Unsupported".into(),
        }
    }

    pub fn to_prisma_type(&self) -> PrismaValueType {
        match self.id {
            TypeIdentifier::String => PrismaValueType::String,
            TypeIdentifier::Int => PrismaValueType::Int,
            TypeIdentifier::BigInt => PrismaValueType::BigInt,
            TypeIdentifier::Float => PrismaValueType::Float,
            TypeIdentifier::Decimal => PrismaValueType::Decimal,
            TypeIdentifier::Boolean => PrismaValueType::Boolean,
            TypeIdentifier::Enum(id) => PrismaValueType::Enum(self.dm.walk(id).name().to_owned()),
            TypeIdentifier::UUID => PrismaValueType::String,
            TypeIdentifier::Json => PrismaValueType::Object,
            TypeIdentifier::DateTime => PrismaValueType::Date,
            TypeIdentifier::Bytes => PrismaValueType::Bytes,
            TypeIdentifier::Unsupported => PrismaValueType::Any,
        }
    }
}

impl std::fmt::Debug for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TypeIdentifier")
            .field(&format!("{:?}", self.id))
            .finish()
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

pub struct FieldTypeInformation {
    pub typ: Type,
    pub arity: FieldArity,
    pub native_type: Option<NativeTypeInstance>,
}

impl FieldTypeInformation {
    pub fn to_prisma_type(&self) -> PrismaValueType {
        let type_ = match (self.typ.id, self.native_type.as_ref()) {
            (TypeIdentifier::DateTime, Some(native_type))
                if native_type.name() == "Time" || native_type.name() == "Timetz" =>
            {
                PrismaValueType::Time
            }
            _ => self.typ.to_prisma_type(),
        };
        if self.arity.is_list() {
            PrismaValueType::Array(Box::new(type_))
        } else {
            type_
        }
    }
}

impl From<Type> for FieldTypeInformation {
    fn from(typ: Type) -> Self {
        FieldTypeInformation {
            typ,
            native_type: None,
            arity: FieldArity::Required,
        }
    }
}
