use crate::TestError;
use std::convert::TryFrom;

#[derive(Debug, Clone)]
pub enum RelationField {
    ParentList(ParentList),
    ChildList(ChildList),
}

impl RelationField {
    pub fn is_required(&self) -> bool {
        match self {
            RelationField::ParentList(x) => x.is_required,
            RelationField::ChildList(x) => x.is_required,
        }
    }

    pub fn is_list(&self) -> bool {
        match self {
            RelationField::ParentList(x) => x.is_list,
            RelationField::ChildList(x) => x.is_list,
        }
    }

    pub fn field(&self) -> String {
        match self {
            RelationField::ParentList(x) => x.field.to_owned(),
            RelationField::ChildList(x) => x.field.to_owned(),
        }
    }

    pub fn optional_suffix(&self) -> &str {
        let field = match self {
            RelationField::ParentList(x) => &x.field,
            RelationField::ChildList(x) => &x.field,
        };

        if field.ends_with("?") {
            "?"
        } else {
            ""
        }
    }
}

impl TryFrom<&str> for RelationField {
    type Error = TestError;

    fn try_from(name: &str) -> Result<Self, Self::Error> {
        let rel_field = match name {
            "ParentList" => Self::ParentList(ParentList::new()),
            "ChildList" => Self::ChildList(ChildList::new()),
            _ => return Err(TestError::parse_error(format!("Unknown relation field `{}`", name))),
        };

        Ok(rel_field)
    }
}

#[derive(Debug, Clone)]
pub struct ParentList {
    pub field: String,
    pub is_list: bool,
    pub is_required: bool,
}

impl ParentList {
    pub fn new() -> Self {
        Self {
            field: "parentsOpt Parent[]".to_string(),
            is_list: true,
            is_required: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChildList {
    pub field: String,
    pub is_list: bool,
    pub is_required: bool,
}

impl ChildList {
    pub fn new() -> Self {
        Self {
            field: "childrenOpt Child[]".to_string(),
            is_list: true,
            is_required: false,
        }
    }
}
