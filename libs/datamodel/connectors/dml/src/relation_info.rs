use enumflags2::BitFlags;
use std::fmt;

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
    pub on_delete: Option<ReferentialAction>,
    /// A strategy indicating what happens when
    /// a related node is deleted.
    pub on_update: Option<ReferentialAction>,
}

impl PartialEq for RelationInfo {
    //ignores the relation name for reintrospection
    fn eq(&self, other: &Self) -> bool {
        self.to == other.to
            && self.fields == other.fields
            && self.references == other.references
            && self.on_delete == other.on_delete
            && self.on_update == other.on_update
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
            on_delete: None,
            on_update: None,
        }
    }
}

/// Describes what happens when related nodes are deleted.
#[repr(u8)]
#[derive(Debug, Copy, PartialEq, Clone, BitFlags)]
pub enum ReferentialAction {
    /// Deletes record if dependent record is deleted. Updates relation scalar
    /// fields if referenced scalar fields of the dependent record are updated.
    /// Prevents operation (both updates and deletes) from succeeding if any
    /// records are connected. This behavior will always result in a runtime
    /// error for required relations.
    Cascade = 1 << 0,
    /// Prevents operation (both updates and deletes) from succeeding if any
    /// records are connected. This behavior will always result in a runtime
    /// error for required relations.
    Restrict = 1 << 1,
    /// Behavior is database specific. Either defers throwing an integrity check
    /// error until the end of the transaction or errors immediately. If
    /// deferred, this makes it possible to temporarily violate integrity in a
    /// transaction while making sure that subsequent operations in the
    /// transaction restore integrity.
    NoAction = 1 << 2,
    /// Sets relation scalar fields to null if the relation is deleted or
    /// updated. This will always result in a runtime error if one or more of the
    /// relation scalar fields are required.
    SetNull = 1 << 3,
    /// Sets relation scalar fields to their default values on update or delete
    /// of relation. Will always result in a runtime error if no defaults are
    /// provided for any relation scalar fields.
    SetDefault = 1 << 4,
}

impl fmt::Display for ReferentialAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReferentialAction::Cascade => write!(f, "Cascade"),
            ReferentialAction::Restrict => write!(f, "Restrict"),
            ReferentialAction::NoAction => write!(f, "NoAction"),
            ReferentialAction::SetNull => write!(f, "SetNull"),
            ReferentialAction::SetDefault => write!(f, "SetDefault"),
        }
    }
}
