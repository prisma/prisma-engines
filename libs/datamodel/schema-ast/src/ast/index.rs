use std::fmt;

#[derive(Debug, Clone, Copy)]
pub enum IndexSortOrder {
    Asc,
    Desc,
}

#[derive(Debug, Clone)]
pub enum IndexFieldPointer<'ast> {
    Scalar(&'ast str),
    InComposite(Vec<&'ast str>),
}

impl<'ast> From<&'ast str> for IndexFieldPointer<'ast> {
    fn from(s: &'ast str) -> Self {
        if !s.contains('.') {
            Self::Scalar(s)
        } else {
            Self::InComposite(s.split('.').collect())
        }
    }
}

impl<'ast> fmt::Display for IndexFieldPointer<'ast> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IndexFieldPointer::Scalar(s) => f.write_str(s),
            IndexFieldPointer::InComposite(s) => f.write_str(&s.join(".")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IndexFieldOptions<'ast> {
    pub name: &'ast str,
    pub sort_order: Option<IndexSortOrder>,
    pub length: Option<u32>,
}

impl<'ast> IndexFieldOptions<'ast> {
    pub fn new(name: impl Into<IndexFieldPointer<'ast>>) -> Self {
        Self {
            field_pointer: name.into(),
            sort_order: None,
            length: None,
        }
    }
}
