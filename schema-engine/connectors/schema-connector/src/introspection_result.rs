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
    pub data_model: String,
    /// The introspected data model is empty
    pub is_empty: bool,
    /// Introspection warnings
    pub warnings: Option<String>,
    /// The database view definitions. None if preview feature
    /// is not enabled.
    pub views: Option<Vec<ViewDefinition>>,
}

/// The result structure from a successful introspection run for multiple schema files.
pub struct IntrospectionMultiResult {
    /// Datamodels
    pub datamodels: Vec<(String, String)>,
    /// The introspected data model is empty
    pub is_empty: bool,
    /// Introspection warnings
    pub warnings: Option<String>,
    /// The database view definitions. None if preview feature
    /// is not enabled.
    pub views: Option<Vec<ViewDefinition>>,
}

impl From<IntrospectionMultiResult> for IntrospectionResult {
    fn from(res: IntrospectionMultiResult) -> Self {
        let data_model = match res.datamodels.is_empty() {
            true => String::new(),
            false => res.datamodels.into_iter().next().unwrap().1,
        };

        Self {
            data_model,
            is_empty: res.is_empty,
            warnings: res.warnings,
            views: res.views,
        }
    }
}
