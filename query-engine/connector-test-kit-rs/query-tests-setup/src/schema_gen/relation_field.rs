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

    pub fn is_to_one_opt(&self) -> bool {
        matches!(self, Self::ToOneOpt { .. })
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
                true => "Parent",
                false => "Child",
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
                    "Unknown relation field `{name}`. Valid names are: ToOneOpt, ToOneReq and ToMany"
                )));
            }
        };

        Ok(rel_field)
    }
}
