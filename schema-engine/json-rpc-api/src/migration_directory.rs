use crate::js_result::JsResult;
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use tsify::Tsify;

/// Information about a migration file within a migration directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
pub struct MigrationFile {
    /// Relative path to the migration file from the migration directory.
    /// E.g., `migration.sql`.
    pub path: String,

    /// Content of the migration file or error if it couldn't be read.
    pub content: JsResult<String, String>,
}

/// Information about a migration directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub struct MigrationDirectory {
    /// Relative path to a migration directory from `baseDir`.
    /// E.g., `20201117144659_test`.
    pub path: String,

    /// Information about the migration file within the directory.
    pub migration_file: MigrationFile,
}

impl MigrationDirectory {
    /// The `{timestamp}_{name}` formatted migration name.
    pub fn migration_name(&self) -> &str {
        self.path.as_str()
    }
}
