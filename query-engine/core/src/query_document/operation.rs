use super::Selection;

#[derive(Debug, Clone)]
pub enum Operation {
    Read(Selection),
    Write(Selection),
}

impl Operation {
    pub fn is_find_one(&self) -> bool {
        match self {
            Self::Read(selection) => selection.is_find_one(),
            _ => false,
        }
    }

    pub fn into_read(self) -> Option<Selection> {
        match self {
            Self::Read(sel) => Some(sel),
            _ => None,
        }
    }

    pub fn into_write(self) -> Option<Selection> {
        match self {
            Self::Write(sel) => Some(sel),
            _ => None,
        }
    }
}

impl Operation {
    pub fn dedup_selections(self) -> Self {
        match self {
            Self::Read(s) => Self::Read(s.dedup()),
            Self::Write(s) => Self::Write(s.dedup()),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Read(s) => s.name(),
            Self::Write(s) => s.name(),
        }
    }

    pub fn nested_selections(&self) -> &[Selection] {
        match self {
            Self::Read(s) => s.nested_selections(),
            Self::Write(s) => s.nested_selections(),
        }
    }
}
