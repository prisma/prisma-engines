use super::*;
use crate::error::DatamodelError;

/// Represents a strategy for generating IDs.
#[derive(Debug, Copy, PartialEq, Clone)]
pub enum IdStrategy {
    Auto,
    None,
}

impl Parsable for IdStrategy {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "AUTO" => Some(IdStrategy::Auto),
            "NONE" => Some(IdStrategy::None),
            _ => None,
        }
    }

    fn descriptor() -> &'static str {
        "id strategy"
    }
}

impl ToString for IdStrategy {
    fn to_string(&self) -> String {
        match self {
            IdStrategy::Auto => String::from("AUTO"),
            IdStrategy::None => String::from("NONE"),
        }
    }
}

/// Represents a sequence. Can be used to seed IDs.
#[derive(Debug, PartialEq, Clone)]
pub struct Sequence {
    /// The name of the sequence.
    pub name: String,
    /// The initial value of the sequence.
    pub initial_value: i32,
    /// The allocation size of the sequence.
    pub allocation_size: i32,
}

impl WithName for Sequence {
    fn name(&self) -> &String {
        &self.name
    }
    fn set_name(&mut self, name: &str) {
        self.name = String::from(name)
    }
}
