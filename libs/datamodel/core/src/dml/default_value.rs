use super::*;
use crate::{ast, error::DatamodelError};
use std::{convert::TryFrom, fmt};

#[derive(Clone, PartialEq)]
pub enum DefaultValue {
    Single(ScalarValue),
    Expression(ValueGenerator),
}

#[derive(Clone)]
pub struct ValueGenerator {
    pub name: String,
    pub args: Vec<ScalarValue>,

    generator: ValueGeneratorFn,
}

impl ValueGenerator {
    pub fn new(name: String, args: Vec<ScalarValue>) -> std::result::Result<Self, DatamodelError> {
        let generator = ValueGeneratorFn::try_from(name.as_ref())?;

        Ok(ValueGenerator { name, args, generator })
    }

    pub fn return_type(&self) -> ScalarType {
        self.generator.return_type()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn args(&self) -> &[ScalarValue] {
        &self.args
    }

    fn generate(&self) -> Option<ScalarValue> {
        self.generator.invoke()
    }
}

#[derive(Clone, Copy)]
pub enum ValueGeneratorFn {
    UUID,
    CUID,
    Now,
    Autoincrement,
}

impl ValueGeneratorFn {
    pub fn return_type(&self) -> ScalarType {
        match self {
            Self::UUID => ScalarType::String,
            Self::CUID => ScalarType::String,
            Self::Now => ScalarType::DateTime,
            Self::Autoincrement => ScalarType::Int,
        }
    }

    pub fn invoke(&self) -> Option<ScalarValue> {
        match self {
            Self::UUID => Self::generate_uuid(),
            Self::CUID => Self::generate_cuid(),
            Self::Now => Self::generate_now(),
            Self::Autoincrement => None,
        }
    }

    fn generate_cuid() -> Option<ScalarValue> {
        todo!()
    }

    fn generate_uuid() -> Option<ScalarValue> {
        todo!()
    }

    fn generate_now() -> Option<ScalarValue> {
        todo!()
    }
}

impl TryFrom<&str> for ValueGeneratorFn {
    type Error = DatamodelError;

    fn try_from(s: &str) -> std::result::Result<Self, DatamodelError> {
        todo!()
    }
}

impl PartialEq for ValueGenerator {
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name() && self.args() == other.args()
    }
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

    pub fn get_type(&self) -> ScalarType {
        match self {
            Self::Single(v) => v.get_type(),
            Self::Expression(vg) => vg.return_type(),
        }
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

impl TryFrom<ScalarValue> for DefaultValue {
    type Error = DatamodelError;

    fn try_from(sv: ScalarValue) -> std::result::Result<Self, DatamodelError> {
        Ok(match sv {
            ScalarValue::Expression(name, _, args) => Self::Expression(ValueGenerator::new(name, args)?),
            other => Self::Single(other),
        })
    }
}

impl Into<ast::Expression> for DefaultValue {
    fn into(self) -> ast::Expression {
        match self {
            Self::Single(v) => v.into(),
            Self::Expression(e) => {
                let exprs = e.args.into_iter().map(Into::into).collect();
                ast::Expression::Function(e.name, exprs, ast::Span::empty())
            }
        }
    }
}
