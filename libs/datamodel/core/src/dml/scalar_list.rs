use crate::Parsable;

/// Represents a strategy for embedding scalar lists.
#[derive(Debug, Copy, PartialEq, Clone)]
pub enum ScalarListStrategy {
    Embedded,
    Relation,
}

impl Parsable for ScalarListStrategy {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "EMBEDDED" => Some(ScalarListStrategy::Embedded),
            "RELATION" => Some(ScalarListStrategy::Relation),
            _ => None,
        }
    }

    fn descriptor() -> &'static str {
        "scalar list strategy"
    }
}

impl ToString for ScalarListStrategy {
    fn to_string(&self) -> String {
        match self {
            ScalarListStrategy::Embedded => String::from("EMBEDDED"),
            ScalarListStrategy::Relation => String::from("RELATION"),
        }
    }
}
