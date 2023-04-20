use super::Selection;
use crate::ArgumentValue;
use schema::QuerySchemaRef;

#[derive(Debug, Clone)]
pub enum Operation {
    Read(Selection),
    Write(Selection),
}

impl Operation {
    pub fn is_find_unique(&self, schema: &QuerySchemaRef) -> bool {
        schema
            .find_query_field(self.name())
            .map(|field| field.is_find_unique())
            .unwrap_or(false)
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

    pub fn into_selection(self) -> Selection {
        match self {
            Operation::Read(selection) => selection,
            Operation::Write(selection) => selection,
        }
    }

    pub fn as_read(&self) -> Option<&Selection> {
        if let Self::Read(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn arguments(&self) -> &[(String, ArgumentValue)] {
        match self {
            Operation::Read(x) => x.arguments(),
            Operation::Write(x) => x.arguments(),
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
