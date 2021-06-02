/// Holds information about a relation field.
/// todo once we handle M2M cleanly this should really become more typed
/// both relation fields of a relation have a Relationinfo but only one has
/// a foreign key. only that side needs fields, references, onDelete and fk_name
/// and for that side none of these are optional
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
    /// Foreign Key Constraint Name if there is one
    pub fk_name: Option<String>,
    /// Whether the Foreign Key Name matches the default for the db
    pub fk_name_matches_default: bool,
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
            fk_name: None,
            fk_name_matches_default: false,
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
