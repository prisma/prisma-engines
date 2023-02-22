use crate::{ScalarFieldRef, ScalarFieldWeak};

#[derive(Debug)]
pub struct Index {
    pub name: Option<String>,
    pub fields: Vec<ScalarFieldWeak>,
    pub typ: IndexType,
}

impl Index {
    pub fn fields(&self) -> Vec<ScalarFieldRef> {
        self.fields.clone()
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum IndexType {
    Unique,
    Normal,
}
