use std::fmt;

use prisma_value::PrismaValue;

#[derive(Debug, PartialEq)]
pub struct Placeholder(String);

impl Placeholder {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn name(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Placeholder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Debug, PartialEq)]
pub struct GeneratorCall {
    name: String,
    args: Vec<PrismaValue>,
}

impl GeneratorCall {
    pub fn new(name: String, args: Vec<PrismaValue>) -> Self {
        Self { name, args }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn args(&self) -> &[PrismaValue] {
        &self.args
    }
}

impl fmt::Display for GeneratorCall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(", self.name())?;
        for (i, arg) in self.args.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", arg)?;
        }
        write!(f, ")")
    }
}
