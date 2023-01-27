use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    pub data_model: String,
    /// The introspected data model is empty
    pub is_empty: bool,
    /// Introspection warnings
    pub warnings: Vec<Warning>,
    /// The database view definitions. None if preview feature
    /// is not enabled.
    pub views: Option<Vec<ViewDefinition>>,
}

/// A warning, spawned from an introspection run.
#[derive(Serialize, Deserialize, Debug)]
pub struct Warning {
    /// A unique indentifying code for the warning.
    pub code: u32,
    /// The warning message.
    pub message: String,
    /// The affected items that triggered this warning.
    pub affected: Value,
}

/// The output type from introspection.
#[derive(Debug, Deserialize, Serialize)]
pub struct IntrospectionResultOutput {
    /// Datamodel
    pub datamodel: String,
    /// warnings
    pub warnings: Vec<Warning>,
    /// views
    pub views: Option<Vec<ViewDefinition>>,
}
