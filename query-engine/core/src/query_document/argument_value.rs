use bigdecimal::BigDecimal;
use indexmap::IndexMap;
use prisma_models::PrismaValue;

pub type ArgumentValueObject = IndexMap<String, ArgumentValue>;

/// Represents the input values in a Document.
/// This abstraction is mostly there to hold special kind of values such as `FieldRef` which have to be disambiguated at query-validation time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgumentValue {
    Scalar(PrismaValue),
    Object(ArgumentValueObject),
    List(Vec<ArgumentValue>),
}

impl ArgumentValue {
    pub fn null() -> Self {
        Self::Scalar(PrismaValue::Null)
    }

    pub fn int(i: i64) -> Self {
        Self::Scalar(PrismaValue::Int(i))
    }

    pub fn float(dec: BigDecimal) -> Self {
        Self::Scalar(PrismaValue::Float(dec))
    }

    pub fn string(str: String) -> Self {
        Self::Scalar(PrismaValue::String(str))
    }

    pub fn bool(bool: bool) -> Self {
        Self::Scalar(PrismaValue::Boolean(bool))
    }

    pub fn r#enum(str: String) -> Self {
        Self::Scalar(PrismaValue::Enum(str))
    }

    pub fn object(obj: impl Into<ArgumentValueObject>) -> Self {
        Self::Object(obj.into())
    }

    pub fn list(values: impl Into<Vec<ArgumentValue>>) -> Self {
        Self::List(values.into())
    }

    pub fn into_object(self) -> Option<ArgumentValueObject> {
        match self {
            Self::Object(obj) => Some(obj),
            _ => None,
        }
    }
}

impl From<PrismaValue> for ArgumentValue {
    fn from(value: PrismaValue) -> Self {
        match value {
            PrismaValue::List(list) => Self::List(list.into_iter().map(ArgumentValue::from).collect()),
            PrismaValue::Object(obj) => {
                Self::Object(obj.into_iter().map(|(k, v)| (k, ArgumentValue::from(v))).collect())
            }
            _ => Self::Scalar(value),
        }
    }
}
