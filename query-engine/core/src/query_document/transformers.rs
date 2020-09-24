//! Transformations for the parsed query document tree.
//! As the schema validation guarantees the presence, type conformity, etc. of incoming documents,
//! consumers of the parsed query document want to directly unwrap and access the incoming data,
//! but would need to clutter their code with tons of matches and unwraps.
//! The transformers in this file helps consumers to directly access the data in the shape they
//! assume the data has to be because of the structural guarantees of the query schema validation.
use super::*;
use chrono::prelude::*;
use prisma_models::{OrderBy, PrismaValue, ScalarFieldRef};
use rust_decimal::prelude::ToPrimitive;
use std::convert::TryInto;

impl TryInto<PrismaValue> for ParsedInputValue {
    type Error = QueryParserError;

    fn try_into(self) -> QueryParserResult<PrismaValue> {
        match self {
            ParsedInputValue::Single(val) => Ok(val),
            ParsedInputValue::List(values) => values
                .into_iter()
                .map(|val| val.try_into())
                .collect::<QueryParserResult<Vec<PrismaValue>>>()
                .map(|vec| PrismaValue::List(vec)),

            v => Err(QueryParserError {
                path: QueryPath::default(),
                error_kind: QueryParserErrorKind::AssertionError(format!(
                    "Attempted conversion of ParsedInputValue ({:?}) into PrismaValue failed.",
                    v
                )),
            }),
        }
    }
}

impl TryInto<ParsedInputMap> for ParsedInputValue {
    type Error = QueryParserError;

    fn try_into(self) -> QueryParserResult<ParsedInputMap> {
        match self {
            ParsedInputValue::Map(val) => Ok(val),
            v => Err(QueryParserError {
                path: QueryPath::default(),
                error_kind: QueryParserErrorKind::AssertionError(format!(
                    "Attempted conversion of non-map ParsedInputValue ({:?}) into map failed.",
                    v
                )),
            }),
        }
    }
}

impl TryInto<Option<ParsedInputMap>> for ParsedInputValue {
    type Error = QueryParserError;

    fn try_into(self) -> QueryParserResult<Option<ParsedInputMap>> {
        match self {
            ParsedInputValue::Single(PrismaValue::Null) => Ok(None),
            ParsedInputValue::Map(val) => Ok(Some(val)),
            v => Err(QueryParserError {
                path: QueryPath::default(),
                error_kind: QueryParserErrorKind::AssertionError(format!(
                    "Attempted conversion of non-map ParsedInputValue ({:?}) into Option map failed.",
                    v
                )),
            }),
        }
    }
}

impl TryInto<Vec<ParsedInputValue>> for ParsedInputValue {
    type Error = QueryParserError;

    fn try_into(self) -> QueryParserResult<Vec<ParsedInputValue>> {
        match self {
            ParsedInputValue::List(vals) => Ok(vals),
            v => Err(QueryParserError {
                path: QueryPath::default(),
                error_kind: QueryParserErrorKind::AssertionError(format!(
                    "Attempted conversion of non-list ParsedInputValue ({:?}) into list failed.",
                    v
                )),
            }),
        }
    }
}

impl TryInto<Option<String>> for ParsedInputValue {
    type Error = QueryParserError;

    fn try_into(self) -> QueryParserResult<Option<String>> {
        let prisma_value: PrismaValue = self.try_into()?;

        match prisma_value {
            PrismaValue::String(s) => Ok(Some(s)),
            PrismaValue::Enum(s) => Ok(Some(s)),
            PrismaValue::Null => Ok(None),
            v => Err(QueryParserError {
                path: QueryPath::default(),
                error_kind: QueryParserErrorKind::AssertionError(format!(
                    "Attempted conversion of non-String Prisma value type ({:?}) into String failed.",
                    v
                )),
            }),
        }
    }
}

impl TryInto<OrderBy> for ParsedInputValue {
    type Error = QueryParserError;

    fn try_into(self) -> QueryParserResult<OrderBy> {
        match self {
            Self::OrderBy(ord) => Ok(ord),
            v => Err(QueryParserError {
                path: QueryPath::default(),
                error_kind: QueryParserErrorKind::AssertionError(format!(
                    "Attempted conversion of non-order-by enum ({:?}) into order by enum value failed.",
                    v
                )),
            }),
        }
    }
}

impl TryInto<ScalarFieldRef> for ParsedInputValue {
    type Error = QueryParserError;

    fn try_into(self) -> QueryParserResult<ScalarFieldRef> {
        match self {
            Self::ScalarField(f) => Ok(f),
            v => Err(QueryParserError {
                path: QueryPath::default(),
                error_kind: QueryParserErrorKind::AssertionError(format!(
                    "Attempted conversion of non-field-ref enum ({:?}) into scalar field reference value failed.",
                    v
                )),
            }),
        }
    }
}

impl TryInto<Option<f64>> for ParsedInputValue {
    type Error = QueryParserError;

    fn try_into(self) -> QueryParserResult<Option<f64>> {
        let prisma_value: PrismaValue = self.try_into()?;

        match prisma_value {
            PrismaValue::Float(d) => Ok(d.to_f64()),
            PrismaValue::Null => Ok(None),
            v => Err(QueryParserError {
                path: QueryPath::default(),
                error_kind: QueryParserErrorKind::AssertionError(format!(
                    "Attempted conversion of non-float Prisma value type ({:?}) into float failed.",
                    v
                )),
            }),
        }
    }
}

impl TryInto<Option<bool>> for ParsedInputValue {
    type Error = QueryParserError;

    fn try_into(self) -> QueryParserResult<Option<bool>> {
        let prisma_value: PrismaValue = self.try_into()?;

        match prisma_value {
            PrismaValue::Boolean(b) => Ok(Some(b)),
            PrismaValue::Null => Ok(None),
            v => Err(QueryParserError {
                path: QueryPath::default(),
                error_kind: QueryParserErrorKind::AssertionError(format!(
                    "Attempted conversion of non-bool Prisma value type ({:?}) into bool failed.",
                    v
                )),
            }),
        }
    }
}

impl TryInto<Option<DateTime<Utc>>> for ParsedInputValue {
    type Error = QueryParserError;

    fn try_into(self) -> QueryParserResult<Option<DateTime<Utc>>> {
        let prisma_value: PrismaValue = self.try_into()?;

        match prisma_value {
            PrismaValue::DateTime(dt) => Ok(Some(dt)),
            PrismaValue::Null => Ok(None),
            v => Err(QueryParserError {
                path: QueryPath::default(),
                error_kind: QueryParserErrorKind::AssertionError(format!(
                    "Attempted conversion of non-DateTime Prisma value type ({:?}) into DateTime failed.",
                    v
                )),
            }),
        }
    }
}

impl TryInto<Option<i64>> for ParsedInputValue {
    type Error = QueryParserError;

    fn try_into(self) -> QueryParserResult<Option<i64>> {
        let prisma_value: PrismaValue = self.try_into()?;

        match prisma_value {
            PrismaValue::Int(i) => Ok(Some(i)),
            PrismaValue::Null => Ok(None),
            v => Err(QueryParserError {
                path: QueryPath::default(),
                error_kind: QueryParserErrorKind::AssertionError(format!(
                    "Attempted conversion of non-int Prisma value type ({:?}) into int failed.",
                    v
                )),
            }),
        }
    }
}

impl TryInto<bool> for ParsedInputValue {
    type Error = QueryParserError;

    fn try_into(self) -> QueryParserResult<bool> {
        let prisma_value: PrismaValue = self.try_into()?;

        match prisma_value {
            PrismaValue::Boolean(b) => Ok(b),
            v => Err(QueryParserError {
                path: QueryPath::default(),
                error_kind: QueryParserErrorKind::AssertionError(format!(
                    "Attempted conversion of non-boolean Prisma value type ({:?}) into bool failed.",
                    v
                )),
            }),
        }
    }
}
