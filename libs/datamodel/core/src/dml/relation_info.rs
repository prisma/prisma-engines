use super::Parsable;

/// Holds information about a relation field.
#[derive(Debug, PartialEq, Clone)]
pub struct RelationInfo {
    /// The target model of the relation.
    pub to: String,
    /// The target field of the relation.
    pub to_fields: Vec<String>,
    /// The name of the relation. Internally, an empty string signals no name.
    pub name: String,
    /// A strategy indicating what happens when
    /// a related node is deleted.
    pub on_delete: OnDeleteStrategy,
}

impl RelationInfo {
    /// Creates a new relation info for the
    /// given target model.
    pub fn new(to: &str) -> RelationInfo {
        RelationInfo {
            to: String::from(to),
            to_fields: Vec::new(),
            name: String::new(),
            on_delete: OnDeleteStrategy::None,
        }
    }
    pub fn new_with_field(to: &str, to_field: &str) -> RelationInfo {
        RelationInfo {
            to: String::from(to),
            to_fields: vec![String::from(to_field)],
            name: String::new(),
            on_delete: OnDeleteStrategy::None,
        }
    }

    pub fn new_with_fields(to: &str, to_fields: Vec<String>) -> RelationInfo {
        RelationInfo {
            to: String::from(to),
            to_fields,
            name: String::new(),
            on_delete: OnDeleteStrategy::None,
        }
    }
}

/// Describes what happens when related nodes
/// are deleted.
#[derive(Debug, Copy, PartialEq, Clone)]
pub enum OnDeleteStrategy {
    Cascade,
    None,
}

impl Parsable for OnDeleteStrategy {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "CASCADE" => Some(OnDeleteStrategy::Cascade),
            "NONE" => Some(OnDeleteStrategy::None),
            _ => None,
        }
    }

    fn descriptor() -> &'static str {
        "onDelete strategy"
    }
}

impl ToString for OnDeleteStrategy {
    fn to_string(&self) -> String {
        match self {
            OnDeleteStrategy::Cascade => String::from("CASCADE"),
            OnDeleteStrategy::None => String::from("NONE"),
        }
    }
}
