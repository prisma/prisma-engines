use prisma_models::{InternalEnum, OrderBy, ScalarFieldRef};

#[derive(Debug)]
pub enum EnumType {
    /// Enum from the internal data model.
    Internal(InternalEnum),

    /// Enum defined for order by on fields.
    OrderBy(OrderByEnumType),

    /// Enum referencing fields on a model.
    FieldRef(FieldRefEnumType),
}

impl EnumType {
    pub fn name(&self) -> &str {
        match self {
            Self::Internal(i) => &i.name,
            Self::OrderBy(ord) => &ord.name,
            Self::FieldRef(f) => &f.name,
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

#[derive(Debug, Clone)]
pub struct FieldRefEnumType {
    pub name: String,
    pub values: Vec<(String, ScalarFieldRef)>,
}

impl FieldRefEnumType {
    /// Attempts to find an enum value for the given value key.
    pub fn value_for(&self, name: &str) -> Option<&ScalarFieldRef> {
        self.values
            .iter()
            .find_map(|val| if &val.0 == name { Some(&val.1) } else { None })
    }

    pub fn values(&self) -> Vec<String> {
        self.values.iter().map(|(name, _)| name.to_owned()).collect()
    }
}
