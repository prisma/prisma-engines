use crate::scalars::ScalarType;
use chrono::Utc;
use prisma_value::PrismaValue;
use std::fmt;
use uuid::Uuid;

/// Represents a default specified on a field.
#[derive(Clone, PartialEq)]
pub enum DefaultValue {
    /// a static value, e.g. `@default(1)`
    Single(PrismaValue),
    /// a dynamic value, e.g. `@default(uuid())`
    Expression(ValueGenerator),
}

impl DefaultValue {
    /// Returns either a copy of the contained single value or produces a new
    /// value as defined by the expression.
    pub fn get(&self) -> Option<PrismaValue> {
        match self {
            Self::Single(v) => Some(v.clone()),
            Self::Expression(g) => g.generate(),
        }
    }

    pub fn new_db_generated() -> Self {
        DefaultValue::Expression(ValueGenerator::new_dbgenerated())
    }
}

#[derive(Clone)]
pub struct ValueGenerator {
    pub name: String,
    pub args: Vec<PrismaValue>,
    pub generator: ValueGeneratorFn,
}

impl ValueGenerator {
    pub fn new(name: String, args: Vec<PrismaValue>) -> std::result::Result<Self, String> {
        let generator = ValueGeneratorFn::new(name.as_ref())?;

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

    pub fn new_cuid() -> Self {
        ValueGenerator::new("cuid".to_owned(), vec![]).unwrap()
    }

    pub fn new_uuid() -> Self {
        ValueGenerator::new("uuid".to_owned(), vec![]).unwrap()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn args(&self) -> &[PrismaValue] {
        &self.args
    }

    pub fn generate(&self) -> Option<PrismaValue> {
        self.generator.invoke()
    }

    pub fn check_compatibility_with_scalar_type(&self, scalar_type: ScalarType) -> std::result::Result<(), String> {
        if self.generator.can_handle(scalar_type) {
            Ok(())
        } else {
            Err(format!(
                "The function `{}()` can not be used on fields of type `{}`.",
                &self.name,
                scalar_type.to_string()
            ))
        }
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
    fn new(name: &str) -> std::result::Result<Self, String> {
        match name {
            "cuid" => Ok(Self::CUID),
            "uuid" => Ok(Self::UUID),
            "now" => Ok(Self::Now),
            "autoincrement" => Ok(Self::Autoincrement),
            "dbgenerated" => Ok(Self::DbGenerated),
            _ => Err(format!("The function {} is not a known function.", name)),
        }
    }

    fn invoke(&self) -> Option<PrismaValue> {
        match self {
            Self::UUID => Self::generate_uuid(),
            Self::CUID => Self::generate_cuid(),
            Self::Now => Self::generate_now(),
            Self::Autoincrement => None,
            Self::DbGenerated => None,
        }
    }

    fn can_handle(&self, scalar_type: ScalarType) -> bool {
        match (self, scalar_type) {
            (Self::UUID, ScalarType::String) => true,
            (Self::CUID, ScalarType::String) => true,
            (Self::Now, ScalarType::DateTime) => true,
            (Self::Autoincrement, ScalarType::Int) => true,
            (Self::DbGenerated, _) => true,
            _ => false,
        }
    }

    fn generate_cuid() -> Option<PrismaValue> {
        Some(PrismaValue::String(cuid::cuid().unwrap()))
    }

    fn generate_uuid() -> Option<PrismaValue> {
        Some(PrismaValue::Uuid(Uuid::new_v4()))
    }

    fn generate_now() -> Option<PrismaValue> {
        Some(PrismaValue::DateTime(Utc::now().into()))
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
