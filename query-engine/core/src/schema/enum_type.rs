use prisma_models::{InternalEnum, OrderBy};
// use serde::{Deserialize, Deserializer, Serialize, Serializer};

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
}

// impl Serialize for EnumValue {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         serializer.serialize_str(&*self.as_string())
//     }
// }

// impl<'de> Deserialize<'de> for EnumValue {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         deserializer.deserialize_any(EnumValueVisitor)
//     }
// }

// /// Custom deserialization
// struct EnumValueVisitor;

// impl<'de> Visitor<'de> for EnumValueVisitor {
//     type Value = EnumValue;

//     fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
//         formatter.write_str("A string.")
//     }

//     fn visit_str<E>(self, value: &str) -> Result<EnumValue, E>
//     where
//         E: de::Error,
//     {
//         Ok(EnumValue::string(value.to_owned(), value.to_owned()))
//     }
// }

impl From<InternalEnum> for EnumType {
    fn from(internal_enum: InternalEnum) -> EnumType {
        EnumType::Internal(internal_enum)
    }
}
