//! Transformations for the parsed query document tree.
//! As the schema validation guarantees the presence, type conformity, etc. of incoming documents,
//! consumers of the parsed query document want to directly unwrap and access the incoming data,
//! but would need to clutter their code with tons of matches and unwraps.
//! The transformers in this file helps consumers to directly access the data in the shape they
//! assume the data has to be because of the structural guarantees of the query schema validation.
use super::*;
use bigdecimal::ToPrimitive;
use chrono::prelude::*;
use prisma_models::{OrderBy, PrismaValue, ScalarFieldRef};
use std::convert::TryInto;

impl TryFrom<ParsedInputValue> for PrismaValue {
    type Error = QueryParserError;

    fn try_from(value: ParsedInputValue) -> QueryParserResult<PrismaValue> {
        match value {
            ParsedInputValue::Single(val) => Ok(val),
            ParsedInputValue::List(values) => values
                .into_iter()
                .map(|val| val.try_into())
                .collect::<QueryParserResult<Vec<PrismaValue>>>()
                .map(PrismaValue::List),

            ParsedInputValue::Map(map) => Ok(PrismaValue::Object(
                map.into_iter()
                    .map(|(k, v)| Ok((k, v.try_into()?)))
                    .collect::<QueryParserResult<Vec<_>>>()?,
            )),

            v => Err(QueryParserError::Legacy {
                path: QueryPath::default(),
                error_kind: QueryParserErrorKind::AssertionError(format!(
                    "Attempted conversion of ParsedInputValue ({v:?}) into PrismaValue failed."
                )),
            }),
        }
    }
}

impl TryFrom<ParsedInputValue> for ParsedInputMap {
    type Error = QueryParserError;

    fn try_from(value: ParsedInputValue) -> QueryParserResult<ParsedInputMap> {
        match value {
            ParsedInputValue::Map(val) => Ok(val),
            v => Err(QueryParserError::Legacy {
                path: QueryPath::default(),
                error_kind: QueryParserErrorKind::AssertionError(format!(
                    "Attempted conversion of non-map ParsedInputValue ({v:?}) into map failed."
                )),
            }),
        }
    }
}

impl TryFrom<ParsedInputValue> for Option<ParsedInputMap> {
    type Error = QueryParserError;

    fn try_from(value: ParsedInputValue) -> QueryParserResult<Option<ParsedInputMap>> {
        match value {
            ParsedInputValue::Single(PrismaValue::Null) => Ok(None),
            ParsedInputValue::Map(val) => Ok(Some(val)),
            v => Err(QueryParserError::Legacy {
                path: QueryPath::default(),
                error_kind: QueryParserErrorKind::AssertionError(format!(
                    "Attempted conversion of non-map ParsedInputValue ({v:?}) into Option map failed."
                )),
            }),
        }
    }
}

impl TryFrom<ParsedInputValue> for ParsedInputList {
    type Error = QueryParserError;

    fn try_from(value: ParsedInputValue) -> QueryParserResult<Vec<ParsedInputValue>> {
        match value {
            ParsedInputValue::List(vals) => Ok(vals),
            v => Err(QueryParserError::Legacy {
                path: QueryPath::default(),
                error_kind: QueryParserErrorKind::AssertionError(format!(
                    "Attempted conversion of non-list ParsedInputValue ({v:?}) into list failed."
                )),
            }),
        }
    }
}

impl TryFrom<ParsedInputValue> for Vec<PrismaValue> {
    type Error = QueryParserError;

    fn try_from(value: ParsedInputValue) -> QueryParserResult<Vec<PrismaValue>> {
        match value {
            ParsedInputValue::List(vals) => vals
                .into_iter()
                .map(|val| val.try_into())
                .collect::<QueryParserResult<Vec<_>>>(),
            v => Err(QueryParserError::Legacy {
                path: QueryPath::default(),
                error_kind: QueryParserErrorKind::AssertionError(format!(
                    "Attempted conversion of non-list ParsedInputValue ({v:?}) into prisma value list failed."
                )),
            }),
        }
    }
}

impl TryFrom<ParsedInputValue> for Option<String> {
    type Error = QueryParserError;

    fn try_from(value: ParsedInputValue) -> QueryParserResult<Option<String>> {
        let prisma_value = PrismaValue::try_from(value)?;

        match prisma_value {
            PrismaValue::String(s) => Ok(Some(s)),
            PrismaValue::Enum(s) => Ok(Some(s)),
            PrismaValue::Null => Ok(None),
            v => Err(QueryParserError::Legacy {
                path: QueryPath::default(),
                error_kind: QueryParserErrorKind::AssertionError(format!(
                    "Attempted conversion of non-String Prisma value type ({v:?}) into String failed."
                )),
            }),
        }
    }
}

impl TryFrom<ParsedInputValue> for OrderBy {
    type Error = QueryParserError;

    fn try_from(value: ParsedInputValue) -> QueryParserResult<OrderBy> {
        match value {
            ParsedInputValue::OrderBy(ord) => Ok(ord),
            v => Err(QueryParserError::Legacy {
                path: QueryPath::default(),
                error_kind: QueryParserErrorKind::AssertionError(format!(
                    "Attempted conversion of non-order-by enum ({v:?}) into order by enum value failed."
                )),
            }),
        }
    }
}

impl TryFrom<ParsedInputValue> for ScalarFieldRef {
    type Error = QueryParserError;

    fn try_from(value: ParsedInputValue) -> QueryParserResult<ScalarFieldRef> {
        match value {
            ParsedInputValue::ScalarField(f) => Ok(f),
            v => Err(QueryParserError::Legacy {
                path: QueryPath::default(),
                error_kind: QueryParserErrorKind::AssertionError(format!(
                    "Attempted conversion of non-field-ref enum ({v:?}) into scalar field reference value failed."
                )),
            }),
        }
    }
}

impl TryFrom<ParsedInputValue> for Option<f64> {
    type Error = QueryParserError;

    fn try_from(value: ParsedInputValue) -> QueryParserResult<Option<f64>> {
        let prisma_value = PrismaValue::try_from(value)?;

        match prisma_value {
            PrismaValue::Float(d) => Ok(d.to_f64()),
            PrismaValue::Null => Ok(None),
            v => Err(QueryParserError::Legacy {
                path: QueryPath::default(),
                error_kind: QueryParserErrorKind::AssertionError(format!(
                    "Attempted conversion of non-float Prisma value type ({v:?}) into float failed."
                )),
            }),
        }
    }
}

impl TryFrom<ParsedInputValue> for Option<bool> {
    type Error = QueryParserError;

    fn try_from(value: ParsedInputValue) -> QueryParserResult<Option<bool>> {
        let prisma_value = PrismaValue::try_from(value)?;

        match prisma_value {
            PrismaValue::Boolean(b) => Ok(Some(b)),
            PrismaValue::Null => Ok(None),
            v => Err(QueryParserError::Legacy {
                path: QueryPath::default(),
                error_kind: QueryParserErrorKind::AssertionError(format!(
                    "Attempted conversion of non-bool Prisma value type ({v:?}) into bool failed."
                )),
            }),
        }
    }
}

impl TryFrom<ParsedInputValue> for Option<DateTime<FixedOffset>> {
    type Error = QueryParserError;

    fn try_from(value: ParsedInputValue) -> QueryParserResult<Option<DateTime<FixedOffset>>> {
        let prisma_value = PrismaValue::try_from(value)?;

        match prisma_value {
            PrismaValue::DateTime(dt) => Ok(Some(dt)),
            PrismaValue::Null => Ok(None),
            v => Err(QueryParserError::Legacy {
                path: QueryPath::default(),
                error_kind: QueryParserErrorKind::AssertionError(format!(
                    "Attempted conversion of non-DateTime Prisma value type ({v:?}) into DateTime failed."
                )),
            }),
        }
    }
}

impl TryFrom<ParsedInputValue> for Option<i64> {
    type Error = QueryParserError;

    fn try_from(value: ParsedInputValue) -> QueryParserResult<Option<i64>> {
        let prisma_value = PrismaValue::try_from(value)?;

        match prisma_value {
            PrismaValue::Int(i) => Ok(Some(i)),
            PrismaValue::Null => Ok(None),
            v => Err(QueryParserError::Legacy {
                path: QueryPath::default(),
                error_kind: QueryParserErrorKind::AssertionError(format!(
                    "Attempted conversion of non-int Prisma value type ({v:?}) into int failed."
                )),
            }),
        }
    }
}

impl TryFrom<ParsedInputValue> for bool {
    type Error = QueryParserError;

    fn try_from(value: ParsedInputValue) -> QueryParserResult<bool> {
        let prisma_value = PrismaValue::try_from(value)?;

        match prisma_value {
            PrismaValue::Boolean(b) => Ok(b),
            v => Err(QueryParserError::Legacy {
                path: QueryPath::default(),
                error_kind: QueryParserErrorKind::AssertionError(format!(
                    "Attempted conversion of non-boolean Prisma value type ({v:?}) into bool failed."
                )),
            }),
        }
    }
}
