use prisma_models::ScalarField;
use std::sync::Arc;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct JsonFilter {
    pub field: Arc<ScalarField>,
    pub path: String,
    pub is_target_array: bool,
}
