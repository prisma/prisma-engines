use crate::{ScalarFieldRef, ScalarFieldWeak};

#[derive(Debug)]
pub struct Index {
    pub name: Option<String>,
    pub fields: Vec<ScalarFieldWeak>,
    pub typ: IndexType,
}

impl Index {
    pub fn fields(&self) -> Vec<ScalarFieldRef> {
        self.fields.iter().map(|field| field.upgrade().expect("")).collect()
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum IndexType {
    Unique,
    Normal,
}
