use std::fmt::{self, Formatter};

use query_structure::PrismaValue;
use query_template::{Fragment, PlaceholderFormat};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DbQuery {
    #[serde(rename_all = "camelCase")]
    RawSql {
        sql: String,
        #[serde(serialize_with = "serialize::serialize_params")]
        params: Vec<PrismaValue>,
    },
    #[serde(rename_all = "camelCase")]
    TemplateSql {
        fragments: Vec<Fragment>,
        #[serde(serialize_with = "serialize::serialize_params")]
        params: Vec<PrismaValue>,
        placeholder_format: PlaceholderFormat,
    },
}

impl DbQuery {
    pub fn params(&self) -> &Vec<PrismaValue> {
        match self {
            DbQuery::RawSql { params, .. } => params,
            DbQuery::TemplateSql { params, .. } => params,
        }
    }
}

impl fmt::Display for DbQuery {
    /// Should only be used for debugging, unit testing and playground CLI output.
    /// The placeholder syntax does not attempt to match any actual SQL flavour.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            DbQuery::RawSql { sql, .. } => {
                write!(formatter, "{}", sql)?;
            }
            DbQuery::TemplateSql { fragments, .. } => {
                let placeholder_format = PlaceholderFormat {
                    prefix: "$",
                    has_numbering: true,
                };
                let mut number = 1;
                for fragment in fragments {
                    match fragment {
                        Fragment::StringChunk(s) => {
                            write!(formatter, "{}", s)?;
                        }
                        Fragment::Parameter => {
                            placeholder_format.write(formatter, &mut number)?;
                        }
                        Fragment::ParameterTuple => {
                            write!(formatter, "[")?;
                            placeholder_format.write(formatter, &mut number)?;
                            write!(formatter, "]")?;
                        }
                        Fragment::ParameterTupleList => {
                            write!(formatter, "[(")?;
                            placeholder_format.write(formatter, &mut number)?;
                            write!(formatter, ")]")?;
                        }
                    };
                }
            }
        }
        Ok(())
    }
}

mod serialize {
    use query_structure::{PrismaValue, PrismaValueType};
    use serde::{Serialize, Serializer};

    pub(super) fn serialize_params<S: Serializer>(params: &[PrismaValue], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(params.iter().map(TaggedPrismaValue::from))
    }

    #[derive(Serialize)]
    struct TaggedPrismaValue<'a> {
        value: &'a PrismaValue,
        r#type: PrismaValueType,
    }

    impl<'a> From<&'a PrismaValue> for TaggedPrismaValue<'a> {
        fn from(value: &'a PrismaValue) -> Self {
            Self {
                value,
                r#type: value.r#type(),
            }
        }
    }
}
