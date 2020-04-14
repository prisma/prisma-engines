use crate::{ast::Span, error::DatamodelError};
use chrono::Utc;
use prisma_value::PrismaValue;
use std::{convert::TryFrom, fmt};
use uuid::Uuid;

#[derive(Clone, PartialEq)]
pub enum DefaultValue {
    Single(PrismaValue),
    Expression(ValueGenerator),
}

impl DefaultValue {
    // Returns either a copy of the contained value or produces a new
    // value as defined by the expression.
    pub fn get(&self) -> Option<PrismaValue> {
        match self {
            Self::Single(v) => Some(v.clone()),
            Self::Expression(g) => g.generate(),
        }
    }
}

#[derive(Clone)]
pub struct ValueGenerator {
    pub name: String,
    pub args: Vec<PrismaValue>,

    pub generator: ValueGeneratorFn,
}

impl ValueGenerator {
    pub fn new(name: String, args: Vec<PrismaValue>) -> std::result::Result<Self, DatamodelError> {
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

    pub fn generate(&self) -> Option<PrismaValue> {
        self.generator.invoke()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn args(&self) -> &[PrismaValue] {
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
    pub fn invoke(&self) -> Option<PrismaValue> {
        match self {
            Self::UUID => Self::generate_uuid(),
            Self::CUID => Self::generate_cuid(),
            Self::Now => Self::generate_now(),
            Self::Autoincrement => None,
            Self::DbGenerated => None,
        }
    }

    fn generate_cuid() -> Option<PrismaValue> {
        Some(PrismaValue::String(cuid::cuid().unwrap()))
    }

    fn generate_uuid() -> Option<PrismaValue> {
        Some(PrismaValue::Uuid(Uuid::new_v4()))
    }

    fn generate_now() -> Option<PrismaValue> {
        Some(PrismaValue::DateTime(Utc::now()))
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
