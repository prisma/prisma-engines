use crate::{ScalarFieldRef, ScalarFieldWeak};

#[derive(Debug)]
pub struct Index {
    pub name: Option<String>,
    pub fields: Vec<(Vec<String>, ScalarFieldWeak)>,
    pub typ: IndexType,
}

impl Index {
    pub fn fields(&self) -> Vec<(Vec<String>, ScalarFieldRef)> {
        self.fields
            .iter()
            .map(|(path, field)| (path.clone(), field.upgrade().unwrap()))
            .collect()
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum IndexType {
    Unique,
    Normal,
}
