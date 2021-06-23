use crate::TestError;
use std::convert::TryFrom;

#[derive(Debug, Clone)]
pub enum RelationField {
    ParentOpt(ParentOpt),
    ParentReq(ParentReq),
    ParentList(ParentList),
    ChildList(ChildList),
    ChildOpt(ChildOpt),
    ChildReq(ChildReq),
}

impl RelationField {
    pub fn is_required(&self) -> bool {
        match self {
            RelationField::ParentOpt(x) => x.is_required,
            RelationField::ParentReq(x) => x.is_required,
            RelationField::ParentList(x) => x.is_required,
            RelationField::ChildList(x) => x.is_required,
            RelationField::ChildReq(x) => x.is_required,
            RelationField::ChildOpt(x) => x.is_required,
        }
    }

    pub fn is_list(&self) -> bool {
        match self {
            RelationField::ParentOpt(x) => x.is_list,
            RelationField::ParentReq(x) => x.is_list,
            RelationField::ParentList(x) => x.is_list,
            RelationField::ChildOpt(x) => x.is_list,
            RelationField::ChildReq(x) => x.is_list,
            RelationField::ChildList(x) => x.is_list,
        }
    }

    pub fn field_name(&self) -> String {
        match self {
            RelationField::ParentOpt(x) => x.field.to_owned(),
            RelationField::ParentReq(x) => x.field.to_owned(),
            RelationField::ParentList(x) => x.field.to_owned(),
            RelationField::ChildOpt(x) => x.field.to_owned(),
            RelationField::ChildReq(x) => x.field.to_owned(),
            RelationField::ChildList(x) => x.field.to_owned(),
        }
    }

    pub fn type_name(&self) -> String {
        match self {
            RelationField::ParentOpt(x) => x.typ.to_owned(),
            RelationField::ParentReq(x) => x.typ.to_owned(),
            RelationField::ParentList(x) => x.typ.to_owned(),
            RelationField::ChildOpt(x) => x.typ.to_owned(),
            RelationField::ChildReq(x) => x.typ.to_owned(),
            RelationField::ChildList(x) => x.typ.to_owned(),
        }
    }

    pub fn optional_suffix(&self) -> &str {
        let type_name = self.type_name();

        if type_name.ends_with('?') {
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
            "ParentOpt" => Self::ParentOpt(ParentOpt::new()),
            "ParentReq" => Self::ParentReq(ParentReq::new()),
            "ParentList" => Self::ParentList(ParentList::new()),
            "ChildOpt" => Self::ChildOpt(ChildOpt::new()),
            "ChildReq" => Self::ChildReq(ChildReq::new()),
            "ChildList" => Self::ChildList(ChildList::new()),
            _ => return Err(TestError::parse_error(format!("Unknown relation field `{}`", name))),
        };

        Ok(rel_field)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ParentList {
    pub field: String,
    pub typ: String,
    pub is_list: bool,
    pub is_required: bool,
}

impl ParentList {
    pub fn new() -> Self {
        Self {
            field: "parentsOpt".to_string(),
            typ: "Parent[]".to_string(),
            is_list: true,
            is_required: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChildList {
    pub field: String,
    pub typ: String,
    pub is_list: bool,
    pub is_required: bool,
}

impl ChildList {
    pub fn new() -> Self {
        Self {
            field: "childrenOpt".to_string(),
            typ: "Child[]".to_string(),
            is_list: true,
            is_required: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChildReq {
    pub field: String,
    pub typ: String,
    pub is_list: bool,
    pub is_required: bool,
}

impl ChildReq {
    pub fn new() -> Self {
        Self {
            field: "childReq".to_string(),
            typ: "Child".to_string(),
            is_list: false,
            is_required: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChildOpt {
    pub field: String,
    pub typ: String,
    pub is_list: bool,
    pub is_required: bool,
}

impl ChildOpt {
    pub fn new() -> Self {
        Self {
            field: "childOpt".to_string(),
            typ: "Child?".to_string(),
            is_list: false,
            is_required: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ParentOpt {
    pub field: String,
    pub typ: String,
    pub is_list: bool,
    pub is_required: bool,
}

impl ParentOpt {
    pub fn new() -> Self {
        Self {
            field: "parentOpt".to_string(),
            typ: "Parent?".to_string(),
            is_list: false,
            is_required: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ParentReq {
    pub field: String,
    pub typ: String,
    pub is_list: bool,
    pub is_required: bool,
}

impl ParentReq {
    pub fn new() -> Self {
        Self {
            field: "parentReq".to_string(),
            typ: "Parent".to_string(),
            is_list: false,
            is_required: true,
        }
    }
}
