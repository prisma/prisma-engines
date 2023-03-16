#[derive(Debug, Clone)]
pub struct PrimaryKey {
    pub alias: Option<String>,
    pub(crate) fields: Vec<crate::ScalarField>,
}

impl PrimaryKey {
    pub fn fields(&self) -> Vec<crate::ScalarField> {
        self.fields.clone()
    }
}
