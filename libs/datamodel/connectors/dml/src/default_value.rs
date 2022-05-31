use crate::scalars::ScalarType;
use prisma_value::PrismaValue;
use std::fmt;

/// Represents a default specified on a field.
#[derive(Clone, PartialEq)]
pub struct DefaultValue {
    pub kind: DefaultKind,
    pub db_name: Option<String>,
}

/// Represents a default specified on a field.
#[derive(Clone, PartialEq)]
pub enum DefaultKind {
    /// a static value, e.g. `@default(1)`
    Single(PrismaValue),
    /// a dynamic value, e.g. `@default(uuid())`
    Expression(ValueGenerator),
}

impl DefaultKind {
    /// Does this match @default(autoincrement())?
    pub fn is_autoincrement(&self) -> bool {
        matches!(self, DefaultKind::Expression(generator) if generator.name == "autoincrement")
    }

    /// Does this match @default(cuid(_))?
    pub fn is_cuid(&self) -> bool {
        matches!(self, DefaultKind::Expression(generator) if generator.name == "cuid")
    }

    /// Does this match @default(dbgenerated(_))?
    pub fn is_dbgenerated(&self) -> bool {
        matches!(self, DefaultKind::Expression(generator) if generator.name == "dbgenerated")
    }

    /// Does this match @default(now())?
    pub fn is_now(&self) -> bool {
        matches!(self, DefaultKind::Expression(generator) if generator.name == "now")
    }

    /// Does this match @default(uuid(_))?
    pub fn is_uuid(&self) -> bool {
        matches!(self, DefaultKind::Expression(generator) if generator.name == "uuid")
    }

    pub fn unwrap_single(self) -> PrismaValue {
        match self {
            DefaultKind::Single(_) => todo!(),
            _ => panic!("called DefaultValue::unwrap_single() on wrong variant"),
        }
    }
}

impl DefaultValue {
    pub fn as_expression(&self) -> Option<&ValueGenerator> {
        match self.kind {
            DefaultKind::Expression(ref expr) => Some(expr),
            _ => None,
        }
    }

    pub fn as_single(&self) -> Option<&PrismaValue> {
        match self.kind {
            DefaultKind::Single(ref v) => Some(v),
            _ => None,
        }
    }

    pub fn kind(&self) -> &DefaultKind {
        &self.kind
    }

    pub fn into_kind(self) -> DefaultKind {
        self.kind
    }

    pub fn mut_kind(&mut self) -> &mut DefaultKind {
        &mut self.kind
    }

    /// Returns either a copy of the contained single value or produces a new
    /// value as defined by the expression.
    #[cfg(feature = "default_generators")]
    pub fn get(&self) -> Option<PrismaValue> {
        match self.kind {
            DefaultKind::Single(ref v) => Some(v.clone()),
            DefaultKind::Expression(ref g) => g.generate(),
        }
    }

    /// Does this match @default(autoincrement())?
    pub fn is_autoincrement(&self) -> bool {
        self.kind.is_autoincrement()
    }

    /// Does this match @default(cuid(_))?
    pub fn is_cuid(&self) -> bool {
        self.kind.is_cuid()
    }

    /// Does this match @default(dbgenerated(_))?
    pub fn is_dbgenerated(&self) -> bool {
        self.kind.is_dbgenerated()
    }

    /// Does this match @default(now())?
    pub fn is_now(&self) -> bool {
        self.kind.is_now()
    }

    /// Does this match @default(uuid(_))?
    pub fn is_uuid(&self) -> bool {
        self.kind.is_uuid()
    }

    pub fn new_expression(generator: ValueGenerator) -> Self {
        let kind = DefaultKind::Expression(generator);

        Self { kind, db_name: None }
    }

    pub fn new_single(value: PrismaValue) -> Self {
        let kind = DefaultKind::Single(value);

        Self { kind, db_name: None }
    }

    pub fn set_db_name(&mut self, name: impl ToString) {
        self.db_name = Some(name.to_string());
    }

    /// The default value constraint name.
    pub fn db_name(&self) -> Option<&str> {
        self.db_name.as_deref()
    }

    // Returns the dbgenerated function for a default value
    // intended for primary key values!
    pub fn to_dbgenerated_func(&self) -> Option<String> {
        match self.kind {
            DefaultKind::Expression(ref expr) if expr.is_dbgenerated() => expr.args.get(0).map(|val| val.1.to_string()),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct ValueGenerator {
    name: String,
    args: Vec<(Option<String>, PrismaValue)>,
    generator: ValueGeneratorFn,
}

impl ValueGenerator {
    pub fn new(name: String, args: Vec<(Option<String>, PrismaValue)>) -> Result<Self, String> {
        let generator = ValueGeneratorFn::new(name.as_ref())?;

        Ok(ValueGenerator { name, args, generator })
    }

    pub fn new_autoincrement() -> Self {
        ValueGenerator::new("autoincrement".to_owned(), vec![]).unwrap()
    }

    pub fn new_sequence(args: Vec<(Option<String>, PrismaValue)>) -> Self {
        ValueGenerator::new("sequence".to_owned(), args).unwrap()
    }

    pub fn new_dbgenerated(description: String) -> Self {
        if description.trim_matches('\0').is_empty() {
            ValueGenerator::new("dbgenerated".to_owned(), Vec::new()).unwrap()
        } else {
            ValueGenerator::new("dbgenerated".to_owned(), vec![(None, PrismaValue::String(description))]).unwrap()
        }
    }

    pub fn new_auto() -> Self {
        ValueGenerator::new("auto".to_owned(), Vec::new()).unwrap()
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

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn args(&self) -> &[(Option<String>, PrismaValue)] {
        &self.args
    }

    pub fn generator(&self) -> ValueGeneratorFn {
        self.generator
    }

    pub fn as_dbgenerated(&self) -> Option<&str> {
        if !(self.is_dbgenerated()) {
            return None;
        }

        self.args.get(0).and_then(|v| v.1.as_string())
    }

    #[cfg(feature = "default_generators")]
    pub fn generate(&self) -> Option<PrismaValue> {
        self.generator.invoke()
    }

    pub fn check_compatibility_with_scalar_type(&self, scalar_type: ScalarType) -> Result<(), String> {
        if self.generator.can_handle(scalar_type) {
            Ok(())
        } else {
            Err(format!(
                "The function `{}()` cannot be used on fields of type `{}`.",
                &self.name,
                scalar_type.to_string()
            ))
        }
    }

    pub fn is_dbgenerated(&self) -> bool {
        self.name == "dbgenerated"
    }

    pub fn is_autoincrement(&self) -> bool {
        self.name == "autoincrement" || self.name == "sequence"
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ValueGeneratorFn {
    Uuid,
    Cuid,
    Now,
    Autoincrement,
    DbGenerated,
    Auto,
}

impl ValueGeneratorFn {
    fn new(name: &str) -> std::result::Result<Self, String> {
        match name {
            "cuid" => Ok(Self::Cuid),
            "uuid" => Ok(Self::Uuid),
            "now" => Ok(Self::Now),
            "autoincrement" => Ok(Self::Autoincrement),
            "sequence" => Ok(Self::Autoincrement),
            "dbgenerated" => Ok(Self::DbGenerated),
            "auto" => Ok(Self::Auto),
            _ => Err(format!("The function {} is not a known function.", name)),
        }
    }

    #[cfg(feature = "default_generators")]
    fn invoke(&self) -> Option<PrismaValue> {
        match self {
            Self::Uuid => Some(Self::generate_uuid()),
            Self::Cuid => Some(Self::generate_cuid()),
            Self::Now => Some(Self::generate_now()),
            Self::Autoincrement => None,
            Self::DbGenerated => None,
            Self::Auto => None,
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
            (Self::Auto, _) => true,
            _ => false,
        }
    }

    #[cfg(feature = "default_generators")]
    fn generate_cuid() -> PrismaValue {
        PrismaValue::String(cuid::cuid().unwrap())
    }

    #[cfg(feature = "default_generators")]
    fn generate_uuid() -> PrismaValue {
        PrismaValue::Uuid(uuid::Uuid::new_v4())
    }

    #[cfg(feature = "default_generators")]
    fn generate_now() -> PrismaValue {
        PrismaValue::DateTime(chrono::Utc::now().into())
    }
}

impl PartialEq for ValueGenerator {
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name() && self.args() == other.args()
    }
}

impl fmt::Debug for DefaultValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            DefaultKind::Single(ref v) => write!(f, "DefaultValue::Single({:?})", v),
            DefaultKind::Expression(g) => write!(f, "DefaultValue::Expression({}(){:?})", g.name(), g.args),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DefaultValue, ValueGenerator};

    #[test]
    fn default_value_is_autoincrement() {
        let auto_increment_default = DefaultValue::new_expression(ValueGenerator::new_autoincrement());

        assert!(auto_increment_default.is_autoincrement());
    }

    #[test]
    fn default_value_is_now() {
        let now_default = DefaultValue::new_expression(ValueGenerator::new_now());

        assert!(now_default.is_now());
        assert!(!now_default.is_autoincrement());
    }

    #[test]
    fn default_value_is_uuid() {
        let uuid_default = DefaultValue::new_expression(ValueGenerator::new_uuid());

        assert!(uuid_default.is_uuid());
        assert!(!uuid_default.is_autoincrement());
    }

    #[test]
    fn default_value_is_cuid() {
        let cuid_default = DefaultValue::new_expression(ValueGenerator::new_cuid());

        assert!(cuid_default.is_cuid());
        assert!(!cuid_default.is_now());
    }

    #[test]
    fn default_value_is_dbgenerated() {
        let db_generated_default = DefaultValue::new_expression(ValueGenerator::new_dbgenerated("test".to_string()));

        assert!(db_generated_default.is_dbgenerated());
        assert!(!db_generated_default.is_now());
        assert!(!db_generated_default.is_autoincrement());
    }
}
