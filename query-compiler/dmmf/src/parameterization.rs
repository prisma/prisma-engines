//! Parameterization rules for query plan caching.
//!
//! This module defines metadata that tells the JavaScript client which query
//! arguments can be parameterized for caching purposes.

use serde::Serialize;

/// Rules for which query arguments can be parameterized.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterizationRules {
    /// Whether scalar values in filter contexts can be parameterized
    pub scalar_filters: bool,

    /// Whether scalar values in data/mutation contexts can be parameterized
    pub scalar_data: bool,

    /// Whether enum values can be parameterized
    pub enum_values: bool,

    /// Contexts/keys that must NOT be parameterized (they affect query structure)
    pub non_parameterizable: Vec<NonParameterizableContext>,
}

/// A context where parameterization is not allowed
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NonParameterizableContext {
    /// The context type (e.g., "orderBy", "pagination", "mode")
    pub context: String,

    /// Specific field or key that can't be parameterized (None = all in context)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,

    /// The reason this can't be parameterized
    pub reason: String,
}

impl Default for ParameterizationRules {
    fn default() -> Self {
        Self {
            scalar_filters: true,
            scalar_data: true,
            enum_values: true,
            non_parameterizable: vec![
                NonParameterizableContext {
                    context: "orderBy".to_string(),
                    key: None,
                    reason: "Sort direction affects query structure".to_string(),
                },
                NonParameterizableContext {
                    context: "pagination".to_string(),
                    key: Some("take".to_string()),
                    reason: "Limit affects query structure".to_string(),
                },
                NonParameterizableContext {
                    context: "pagination".to_string(),
                    key: Some("skip".to_string()),
                    reason: "Offset affects query structure".to_string(),
                },
                NonParameterizableContext {
                    context: "filter".to_string(),
                    key: Some("mode".to_string()),
                    reason: "Case sensitivity mode affects query structure".to_string(),
                },
                NonParameterizableContext {
                    context: "filter".to_string(),
                    key: Some("relationLoadStrategy".to_string()),
                    reason: "Loading strategy affects query structure".to_string(),
                },
                NonParameterizableContext {
                    context: "selection".to_string(),
                    key: None,
                    reason: "Selection booleans affect query structure".to_string(),
                },
                NonParameterizableContext {
                    context: "distinct".to_string(),
                    key: None,
                    reason: "Distinct fields affect query structure".to_string(),
                },
            ],
        }
    }
}
