use crate::{ScalarFieldRef, ScalarFieldWeak};

#[derive(Debug, Clone)]
pub struct PrimaryKey {
    pub alias: Option<String>,
    pub(crate) fields: Vec<ScalarFieldWeak>,
}

impl PrimaryKey {
    pub fn fields(&self) -> Vec<ScalarFieldRef> {
        self.fields.clone()
    }
}
