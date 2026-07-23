use serde::{Deserialize, Serialize};

/// Defines a view in the database.
#[derive(Debug, Deserialize, Serialize)]
pub struct ViewDefinition {
    /// The database or schema where the view is located.
    pub schema: String,
    /// The name of the view.
    pub name: String,
    /// The database definition of the view.
    pub definition: String,
}

/// The result structure from a successful introspection run.
#[derive(Debug)]
pub struct IntrospectionResult {
    /// Datamodel
    pub datamodels: Vec<(String, String)>,
    /// The introspected data model is empty
    pub is_empty: bool,
    /// Introspection warnings
    pub warnings: Option<String>,
    /// The database view definitions. None if preview feature
    /// is not enabled.
    pub views: Option<Vec<ViewDefinition>>,
}

impl IntrospectionResult {
    /// Consumes the result and returns the first datamodel in the introspection result.
    pub fn into_single_datamodel(mut self) -> String {
        self.datamodels.remove(0).1
    }

    /// Returns the first datamodel in the introspection result.
    pub fn single_datamodel(&self) -> &str {
        &self.datamodels[0].1
    }
}
