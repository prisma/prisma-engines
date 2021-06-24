use crate::TestError;
use std::convert::TryFrom;

#[derive(Debug, Clone)]
pub enum RelationField {
    ToOneOpt { child: bool },
    ToOneReq { child: bool },
    ToMany { child: bool },
}

impl RelationField {
    pub fn is_required(&self) -> bool {
        match self {
            RelationField::ToOneOpt { child: _ } => false,
            RelationField::ToOneReq { child: _ } => true,
            RelationField::ToMany { child: _ } => false,
        }
    }

    pub fn is_list(&self) -> bool {
        match self {
            RelationField::ToOneOpt { child: _ } => false,
            RelationField::ToOneReq { child: _ } => false,
            RelationField::ToMany { child: _ } => true,
        }
    }

    pub fn field_name(&self) -> String {
        match self {
            RelationField::ToOneOpt { child } => match child {
                true => "parentOpt",
                false => "childOpt",
            },
            RelationField::ToOneReq { child } => match child {
                true => "parentReq",
                false => "childReq",
            },
            RelationField::ToMany { child } => match child {
                true => "parentsOpt",
                false => "childrenOpt",
            },
        }
        .to_string()
    }

    pub fn type_name(&self) -> String {
        match self {
            RelationField::ToOneOpt { child } => match child {
                true => "Parent?",
                false => "Child?",
            },
            RelationField::ToOneReq { child } => match child {
                true => "Child",
                false => "Parent",
            },
            RelationField::ToMany { child } => match child {
                true => "Parent[]",
                false => "Child[]",
            },
        }
        .to_string()
    }

    pub fn optional_suffix(&self) -> String {
        match self {
            RelationField::ToOneOpt { child: _ } => "?",
            RelationField::ToOneReq { child: _ } => "",
            RelationField::ToMany { child: _ } => "",
        }
        .to_string()
    }
}

impl TryFrom<(&str, bool)> for RelationField {
    type Error = TestError;

    fn try_from(from: (&str, bool)) -> Result<Self, Self::Error> {
        let (name, child) = from;
        let rel_field = match name {
            "ToOneOpt" => RelationField::ToOneOpt { child },
            "ToOneReq" => RelationField::ToOneReq { child },
            "ToMany" => RelationField::ToMany { child },
            _ => {
                return Err(TestError::parse_error(format!(
                    "Unknown relation field `{}`. Valid names are: ToOneOpt, ToOneReq and ToMany",
                    name
                )))
            }
        };

        Ok(rel_field)
    }
}

// #[derive(Debug, Clone, Default)]
// pub struct ParentList {
//     pub field: String,
//     pub typ: String,
//     pub is_list: bool,
//     pub is_required: bool,
// }

// impl ParentList {
//     pub fn new() -> Self {
//         Self {
//             field: "parentsOpt".to_string(),
//             typ: "Parent[]".to_string(),
//             is_list: true,
//             is_required: false,
//         }
//     }
// }

// #[derive(Debug, Clone, Default)]
// pub struct ChildList {
//     pub field: String,
//     pub typ: String,
//     pub is_list: bool,
//     pub is_required: bool,
// }

// impl ChildList {
//     pub fn new() -> Self {
//         Self {
//             field: "childrenOpt".to_string(),
//             typ: "Child[]".to_string(),
//             is_list: true,
//             is_required: false,
//         }
//     }
// }

// #[derive(Debug, Clone, Default)]
// pub struct ChildReq {
//     pub field: String,
//     pub typ: String,
//     pub is_list: bool,
//     pub is_required: bool,
// }

// impl ChildReq {
//     pub fn new() -> Self {
//         Self {
//             field: "childReq".to_string(),
//             typ: "Child".to_string(),
//             is_list: false,
//             is_required: true,
//         }
//     }
// }

// #[derive(Debug, Clone, Default)]
// pub struct ChildOpt {
//     pub field: String,
//     pub typ: String,
//     pub is_list: bool,
//     pub is_required: bool,
// }

// impl ChildOpt {
//     pub fn new() -> Self {
//         Self {
//             field: "childOpt".to_string(),
//             typ: "Child?".to_string(),
//             is_list: false,
//             is_required: false,
//         }
//     }
// }

// #[derive(Debug, Clone, Default)]
// pub struct ParentOpt {
//     pub field: String,
//     pub typ: String,
//     pub is_list: bool,
//     pub is_required: bool,
// }

// impl ParentOpt {
//     pub fn new() -> Self {
//         Self {
//             field: "parentOpt".to_string(),
//             typ: "Parent?".to_string(),
//             is_list: false,
//             is_required: false,
//         }
//     }
// }

// #[derive(Debug, Clone, Default)]
// pub struct ParentReq {
//     pub field: String,
//     pub typ: String,
//     pub is_list: bool,
//     pub is_required: bool,
// }

// impl ParentReq {
//     pub fn new() -> Self {
//         Self {
//             field: "parentReq".to_string(),
//             typ: "Parent".to_string(),
//             is_list: false,
//             is_required: true,
//         }
//     }
// }
