/// Holds information about a relation field.
#[derive(Debug, Clone)]
pub struct RelationInfo {
    /// The target model of the relation.
    pub to: String,
    /// The fields forming the relation.
    pub fields: Vec<String>,
    /// The target field of the relation a.k.a. `references`
    pub references: Vec<String>,
    /// The name of the relation. Internally, an empty string signals no name.
    pub name: String,
    /// A strategy indicating what happens when
    /// a related node is deleted.
    pub on_delete: OnDeleteStrategy,
}

impl PartialEq for RelationInfo {
    //ignores the relation name for reintrospection
    fn eq(&self, other: &Self) -> bool {
        self.to == other.to
            && self.fields == other.fields
            && self.references == other.references
            && self.on_delete == other.on_delete
    }
}

impl RelationInfo {
    /// Creates a new relation info for the
    /// given target model.
    pub fn new(to: &str) -> RelationInfo {
        RelationInfo {
            to: String::from(to),
            fields: Vec::new(),
            references: Vec::new(),
            name: String::new(),
            on_delete: OnDeleteStrategy::None,
        }
    }
}

/// Describes what happens when related nodes are deleted.
#[derive(Debug, Copy, PartialEq, Clone)]
pub enum OnDeleteStrategy {
    Cascade,
    None,
}

impl ToString for OnDeleteStrategy {
    fn to_string(&self) -> String {
        match self {
            OnDeleteStrategy::Cascade => String::from("CASCADE"),
            OnDeleteStrategy::None => String::from("NONE"),
        }
    }
}
