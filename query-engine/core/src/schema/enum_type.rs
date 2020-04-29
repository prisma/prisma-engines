use prisma_models::{InternalEnum, OrderBy};

#[derive(Debug)]
pub enum EnumType {
    Internal(InternalEnum),
    OrderBy(OrderByEnumType),
}

impl EnumType {
    pub fn name(&self) -> &str {
        match self {
            Self::Internal(i) => &i.name,
            Self::OrderBy(ord) => &ord.name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrderByEnumType {
    pub name: String,

    /// E.g. id_ASC -> OrderBy(Id field, ASC sort order)
    pub values: Vec<(String, OrderBy)>,
}

impl OrderByEnumType {
    /// Attempts to find an enum value for the given value key.
    pub fn value_for(&self, name: &str) -> Option<&OrderBy> {
        self.values
            .iter()
            .find_map(|val| if &val.0 == name { Some(&val.1) } else { None })
    }

    pub fn values(&self) -> Vec<String> {
        self.values.iter().map(|(name, _)| name.to_owned()).collect()
    }
}

impl From<InternalEnum> for EnumType {
    fn from(internal_enum: InternalEnum) -> EnumType {
        EnumType::Internal(internal_enum)
    }
}
