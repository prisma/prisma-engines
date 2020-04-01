use super::*;
use crate::{ast::Span, error::DatamodelError};
use chrono::Utc;
use prisma_value::PrismaValue;
use std::{convert::TryFrom, convert::TryInto, fmt};
use uuid::Uuid;

#[derive(Clone, PartialEq)]
pub enum DefaultValue {
    Single(ScalarValue),
    Expression(ValueGenerator),
}

impl DefaultValue {
    // Returns either a copy of the contained value or produces a new
    // value as defined by the expression.
    pub fn get(&self) -> Option<ScalarValue> {
        match self {
            Self::Single(v) => Some(v.clone()),
            Self::Expression(g) => g.generate(),
        }
    }

    pub fn get_as_prisma_value(&self) -> Option<PrismaValue> {
        self.get().map(|sv| match sv {
            ScalarValue::Boolean(x) => PrismaValue::Boolean(x),
            ScalarValue::Int(x) => PrismaValue::Int(i64::from(x)),
            ScalarValue::Float(x) => x.try_into().expect("Can't convert float to decimal"),
            ScalarValue::String(x) => PrismaValue::String(x.clone()),
            ScalarValue::DateTime(x) => PrismaValue::DateTime(x),
            ScalarValue::Decimal(x) => x.try_into().expect("Can't convert float to decimal"),
            ScalarValue::ConstantLiteral(value) => PrismaValue::Enum(value.clone()),
        })
    }
}

#[derive(Clone)]
pub struct ValueGenerator {
    pub name: String,
    pub args: Vec<ScalarValue>,

    pub generator: ValueGeneratorFn,
}

impl ValueGenerator {
    pub fn new(name: String, args: Vec<ScalarValue>) -> std::result::Result<Self, DatamodelError> {
        let generator = ValueGeneratorFn::try_from(name.as_ref())?;

        Ok(ValueGenerator { name, args, generator })
    }

    pub fn new_autoincrement() -> Self {
        ValueGenerator::new("autoincrement".to_owned(), vec![]).unwrap()
    }

    pub fn new_dbgenerated() -> Self {
        ValueGenerator::new("dbgenerated".to_owned(), vec![]).unwrap()
    }

    pub fn new_now() -> Self {
        ValueGenerator::new("now".to_owned(), vec![]).unwrap()
    }

    pub fn generate(&self) -> Option<ScalarValue> {
        self.generator.invoke()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn args(&self) -> &[ScalarValue] {
        &self.args
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ValueGeneratorFn {
    UUID,
    CUID,
    Now,
    Autoincrement,
    DbGenerated,
}

impl ValueGeneratorFn {
    pub fn invoke(&self) -> Option<ScalarValue> {
        match self {
            Self::UUID => Self::generate_uuid(),
            Self::CUID => Self::generate_cuid(),
            Self::Now => Self::generate_now(),
            Self::Autoincrement => None,
            Self::DbGenerated => None,
        }
    }

    fn generate_cuid() -> Option<ScalarValue> {
        Some(ScalarValue::String(cuid::cuid().unwrap()))
    }

    fn generate_uuid() -> Option<ScalarValue> {
        Some(ScalarValue::String(Uuid::new_v4().to_string()))
    }

    fn generate_now() -> Option<ScalarValue> {
        Some(ScalarValue::DateTime(Utc::now()))
    }
}

impl TryFrom<&str> for ValueGeneratorFn {
    type Error = DatamodelError;

    fn try_from(s: &str) -> std::result::Result<Self, DatamodelError> {
        match s {
            "cuid" => Ok(Self::CUID),
            "uuid" => Ok(Self::UUID),
            "now" => Ok(Self::Now),
            "autoincrement" => Ok(Self::Autoincrement),
            "dbgenerated" => Ok(Self::DbGenerated),
            _ => Err(DatamodelError::new_functional_evaluation_error(
                &format!("The function {} is not a known function.", s),
                Span::empty(),
            )),
        }
    }
}

impl PartialEq for ValueGenerator {
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name() && self.args() == other.args()
    }
}

impl fmt::Debug for DefaultValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Single(ref v) => write!(f, "DefaultValue::Single({:?})", v),
            Self::Expression(g) => write!(f, "DefaultValue::Expression({})", g.name()),
        }
    }
}
