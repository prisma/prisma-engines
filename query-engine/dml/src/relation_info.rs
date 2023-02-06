use psl_core::{parser_database as db, schema_ast::ast};

/// Holds information about a relation field.
#[derive(Debug, PartialEq, Clone)]
pub struct RelationInfo {
    /// The target model of the relation.
    pub referenced_model: ast::ModelId,
    /// The fields forming the relation.
    pub fields: Vec<String>,
    /// The target field of the relation a.k.a. `references`
    pub references: Vec<String>,
    /// The name of the relation. Internally, an empty string signals no name.
    pub name: String,
    /// Foreign Key Constraint Name if there is one
    pub fk_name: Option<String>,
    /// A strategy indicating what happens when
    /// a related node is deleted.
    pub on_delete: Option<db::ReferentialAction>,
    /// A strategy indicating what happens when
    /// a related node is updated.
    pub on_update: Option<db::ReferentialAction>,
}

impl RelationInfo {
    /// Creates a new relation info for the
    /// given target model.
    pub fn new(referenced_model: ast::ModelId) -> RelationInfo {
        RelationInfo {
            referenced_model,
            fields: Vec::new(),
            references: Vec::new(),
            name: String::new(),
            fk_name: None,
            on_delete: None,
            on_update: None,
        }
    }
}
