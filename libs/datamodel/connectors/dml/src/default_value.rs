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
    pub fn as_expression(&self) -> Option<&ValueGenerator> {
        match self {
            DefaultValue::Expression(expr) => Some(expr),
            DefaultValue::Single(_) => None,
        }
    }

    pub fn as_single(&self) -> Option<&PrismaValue> {
        match self {
            DefaultValue::Single(v) => Some(v),
            _ => None,
        }
    }

    /// Returns either a copy of the contained single value or produces a new
    /// value as defined by the expression.
    pub fn get(&self) -> Option<PrismaValue> {
        match self {
            Self::Single(v) => Some(v.clone()),
            Self::Expression(g) => g.generate(),
        }
    }

    /// Does this match @default(autoincrement())?
    pub fn is_autoincrement(&self) -> bool {
        matches!(self, DefaultValue::Expression(generator) if generator.name == "autoincrement")
    }

    /// Does this match @default(cuid(_))?
    pub fn is_cuid(&self) -> bool {
        matches!(self, DefaultValue::Expression(generator) if generator.name == "cuid")
    }

    /// Does this match @default(dbgenerated(_))?
    pub fn is_dbgenerated(&self) -> bool {
        matches!(self, DefaultValue::Expression(generator) if generator.name == "dbgenerated")
    }

    /// Does this match @default(now())?
    pub fn is_now(&self) -> bool {
        matches!(self, DefaultValue::Expression(generator) if generator.name == "now")
    }

    /// Does this match @default(uuid(_))?
    pub fn is_uuid(&self) -> bool {
        matches!(self, DefaultValue::Expression(generator) if generator.name == "uuid")
    }

    pub fn new_db_generated(description: String) -> Self {
        DefaultValue::Expression(ValueGenerator::new_dbgenerated(description))
    }

    pub fn db_generated_description(&self) -> Option<String> {
        match self {
            DefaultValue::Expression(ValueGenerator {
                name,
                args,
                generator: ValueGeneratorFn::DbGenerated,
            }) if name == "dbgenerated" => args.first().map(|x| x.to_string()),
            _ => None,
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
    pub fn new(name: String, args: Vec<PrismaValue>) -> Result<Self, String> {
        let generator = ValueGeneratorFn::new(name.as_ref())?;

        Ok(ValueGenerator { name, args, generator })
    }

    pub fn new_autoincrement() -> Self {
        ValueGenerator::new("autoincrement".to_owned(), vec![]).unwrap()
    }

    pub fn new_dbgenerated(description: String) -> Self {
        ValueGenerator::new("dbgenerated".to_owned(), vec![PrismaValue::String(description)]).unwrap()
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

    pub fn as_dbgenerated(&self) -> Option<&str> {
        if !(self.is_dbgenerated()) {
            return None;
        }

        self.args.get(0).and_then(|v| v.as_string())
    }

    pub fn generate(&self) -> Option<PrismaValue> {
        self.generator.invoke()
    }

    pub fn check_compatibility_with_scalar_type(&self, scalar_type: ScalarType) -> Result<(), String> {
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

    pub fn is_dbgenerated(&self) -> bool {
        self.name == "dbgenerated"
    }

    pub fn is_autoincrement(&self) -> bool {
        self.name == "autoincrement"
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ValueGeneratorFn {
    Uuid,
    Cuid,
    Now,
    Autoincrement,
    DbGenerated,
}

impl ValueGeneratorFn {
    fn new(name: &str) -> std::result::Result<Self, String> {
        match name {
            "cuid" => Ok(Self::Cuid),
            "uuid" => Ok(Self::Uuid),
            "now" => Ok(Self::Now),
            "autoincrement" => Ok(Self::Autoincrement),
            "dbgenerated" => Ok(Self::DbGenerated),
            _ => Err(format!("The function {} is not a known function.", name)),
        }
    }

    fn invoke(&self) -> Option<PrismaValue> {
        match self {
            Self::Uuid => Some(Self::generate_uuid()),
            Self::Cuid => Some(Self::generate_cuid()),
            Self::Now => Some(Self::generate_now()),
            Self::Autoincrement => None,
            Self::DbGenerated => None,
        }
    }

    fn can_handle(&self, scalar_type: ScalarType) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match (self, scalar_type) {
            (Self::Uuid, ScalarType::String) => true,
            (Self::Cuid, ScalarType::String) => true,
            (Self::Now, ScalarType::DateTime) => true,
            (Self::Autoincrement, ScalarType::Int) => true,
            (Self::Autoincrement, ScalarType::BigInt) => true,
            (Self::DbGenerated, _) => true,
            _ => false,
        }
    }

    fn generate_cuid() -> PrismaValue {
        PrismaValue::String(cuid::cuid().unwrap())
    }

    fn generate_uuid() -> PrismaValue {
        PrismaValue::Uuid(Uuid::new_v4())
    }

    fn generate_now() -> PrismaValue {
        PrismaValue::DateTime(Utc::now().into())
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
            Self::Expression(g) => write!(f, "DefaultValue::Expression({}(){:?})", g.name(), g.args),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DefaultValue, ValueGenerator};

    #[test]
    fn default_value_is_autoincrement() {
        let auto_increment_default = DefaultValue::Expression(ValueGenerator::new_autoincrement());

        assert!(auto_increment_default.is_autoincrement());
    }

    #[test]
    fn default_value_is_now() {
        let now_default = DefaultValue::Expression(ValueGenerator::new_now());

        assert!(now_default.is_now());
        assert!(!now_default.is_autoincrement());
    }

    #[test]
    fn default_value_is_uuid() {
        let uuid_default = DefaultValue::Expression(ValueGenerator::new_uuid());

        assert!(uuid_default.is_uuid());
        assert!(!uuid_default.is_autoincrement());
    }

    #[test]
    fn default_value_is_cuid() {
        let cuid_default = DefaultValue::Expression(ValueGenerator::new_cuid());

        assert!(cuid_default.is_cuid());
        assert!(!cuid_default.is_now());
    }

    #[test]
    fn default_value_is_dbgenerated() {
        let db_generated_default = DefaultValue::Expression(ValueGenerator::new_dbgenerated("test".to_string()));

        assert!(db_generated_default.is_dbgenerated());
        assert!(!db_generated_default.is_now());
        assert!(!db_generated_default.is_autoincrement());
    }
}
