//! Transformations for the parsed query document tree.
//! As the schema validation guarantees the presence, type conformity, etc. of incoming documents,
//! consumers of the parsed query document want to directly unwrap and access the incoming data,
//! but would need to clutter their code with tons of matches and unwraps.
//! The transformers in this file helps consumers to directly access the data in the shape they
//! assume the data has to be because of the structural guarantees of the query schema validation.
use super::*;
use bigdecimal::ToPrimitive;
use chrono::prelude::*;
use query_structure::{OrderBy, PrismaValue, RelationLoadStrategy, ScalarFieldRef};
use std::convert::TryInto;
use user_facing_errors::query_engine::validation::ValidationError;

impl<'a> TryFrom<ParsedInputValue<'a>> for PrismaValue {
    type Error = ValidationError;

    fn try_from(value: ParsedInputValue<'a>) -> QueryParserResult<PrismaValue> {
        match value {
            ParsedInputValue::Single(val) => Ok(val),
            ParsedInputValue::List(values) => values
                .into_iter()
                .map(|val| val.try_into())
                .collect::<QueryParserResult<Vec<PrismaValue>>>()
                .map(PrismaValue::List),

            ParsedInputValue::Map(map) => Ok(PrismaValue::Object(
                map.into_iter()
                    .map(|(k, v)| Ok((k.into_owned(), v.try_into()?)))
                    .collect::<QueryParserResult<Vec<_>>>()?,
            )),

            v => Err(ValidationError::unexpected_runtime_error(format!(
                "Attempted conversion of ParsedInputValue ({v:?}) into PrismaValue failed."
            ))),
        }
    }
}

impl<'a> TryFrom<ParsedInputValue<'a>> for ParsedInputMap<'a> {
    type Error = ValidationError;

    fn try_from(value: ParsedInputValue<'a>) -> QueryParserResult<ParsedInputMap<'a>> {
        match value {
            ParsedInputValue::Map(val) => Ok(val),
            v => Err(ValidationError::unexpected_runtime_error(format!(
                "Attempted conversion of non-map ParsedInputValue ({v:?}) into map failed."
            ))),
        }
    }
}

impl<'a> TryFrom<ParsedInputValue<'a>> for Option<ParsedInputMap<'a>> {
    type Error = ValidationError;

    fn try_from(value: ParsedInputValue<'a>) -> QueryParserResult<Option<ParsedInputMap<'a>>> {
        match value {
            ParsedInputValue::Single(PrismaValue::Null) => Ok(None),
            ParsedInputValue::Map(val) => Ok(Some(val)),
            v => Err(ValidationError::unexpected_runtime_error(format!(
                "Attempted conversion of non-map ParsedInputValue ({v:?}) into Option map failed."
            ))),
        }
    }
}

impl<'a> TryFrom<ParsedInputValue<'a>> for ParsedInputList<'a> {
    type Error = ValidationError;

    fn try_from(value: ParsedInputValue<'a>) -> QueryParserResult<Vec<ParsedInputValue<'a>>> {
        match value {
            ParsedInputValue::List(vals) => Ok(vals),
            v => Err(ValidationError::unexpected_runtime_error(format!(
                "Attempted conversion of non-list ParsedInputValue ({v:?}) into list failed."
            ))),
        }
    }
}

impl<'a> TryFrom<ParsedInputValue<'a>> for Vec<PrismaValue> {
    type Error = ValidationError;

    fn try_from(value: ParsedInputValue<'a>) -> QueryParserResult<Vec<PrismaValue>> {
        match value {
            ParsedInputValue::List(vals) => vals
                .into_iter()
                .map(|val| val.try_into())
                .collect::<QueryParserResult<Vec<_>>>(),
            v => Err(ValidationError::unexpected_runtime_error(format!(
                "Attempted conversion of non-list ParsedInputValue ({v:?}) into prisma value list failed."
            ))),
        }
    }
}

impl<'a> TryFrom<ParsedInputValue<'a>> for Option<String> {
    type Error = ValidationError;

    fn try_from(value: ParsedInputValue<'a>) -> QueryParserResult<Option<String>> {
        let prisma_value = PrismaValue::try_from(value)?;

        match prisma_value {
            PrismaValue::String(s) => Ok(Some(s)),
            PrismaValue::Enum(s) => Ok(Some(s)),
            PrismaValue::Null => Ok(None),
            v => Err(ValidationError::unexpected_runtime_error(format!(
                "Attempted conversion of non-String Prisma value type ({v:?}) into String failed."
            ))),
        }
    }
}

impl<'a> TryFrom<ParsedInputValue<'a>> for OrderBy {
    type Error = ValidationError;

    fn try_from(value: ParsedInputValue<'a>) -> QueryParserResult<OrderBy> {
        match value {
            ParsedInputValue::OrderBy(ord) => Ok(ord),
            v => Err(ValidationError::unexpected_runtime_error(format!(
                "Attempted conversion of non-order-by enum ({v:?}) into order by enum value failed."
            ))),
        }
    }
}

impl<'a> TryFrom<ParsedInputValue<'a>> for ScalarFieldRef {
    type Error = ValidationError;

    fn try_from(value: ParsedInputValue<'a>) -> QueryParserResult<ScalarFieldRef> {
        match value {
            ParsedInputValue::ScalarField(f) => Ok(f),
            v => Err(ValidationError::unexpected_runtime_error(format!(
                "Attempted conversion of non-field-ref enum ({v:?}) into scalar field reference value failed."
            ))),
        }
    }
}

impl<'a> TryFrom<ParsedInputValue<'a>> for Option<f64> {
    type Error = ValidationError;

    fn try_from(value: ParsedInputValue<'a>) -> QueryParserResult<Option<f64>> {
        let prisma_value = PrismaValue::try_from(value)?;

        match prisma_value {
            PrismaValue::Float(d) => Ok(d.to_f64()),
            PrismaValue::Null => Ok(None),
            v => Err(ValidationError::unexpected_runtime_error(format!(
                "Attempted conversion of non-float Prisma value type ({v:?}) into float failed."
            ))),
        }
    }
}

impl<'a> TryFrom<ParsedInputValue<'a>> for Option<bool> {
    type Error = ValidationError;

    fn try_from(value: ParsedInputValue<'a>) -> QueryParserResult<Option<bool>> {
        let prisma_value = PrismaValue::try_from(value)?;

        match prisma_value {
            PrismaValue::Boolean(b) => Ok(Some(b)),
            PrismaValue::Null => Ok(None),
            v => Err(ValidationError::unexpected_runtime_error(format!(
                "Attempted conversion of non-bool Prisma value type ({v:?}) into bool failed."
            ))),
        }
    }
}

impl<'a> TryFrom<ParsedInputValue<'a>> for Option<DateTime<FixedOffset>> {
    type Error = ValidationError;

    fn try_from(value: ParsedInputValue<'a>) -> QueryParserResult<Option<DateTime<FixedOffset>>> {
        let prisma_value = PrismaValue::try_from(value)?;

        match prisma_value {
            PrismaValue::DateTime(dt) => Ok(Some(dt)),
            PrismaValue::Null => Ok(None),
            v => Err(ValidationError::unexpected_runtime_error(format!(
                "Attempted conversion of non-DateTime Prisma value type ({v:?}) into DateTime failed."
            ))),
        }
    }
}

impl<'a> TryFrom<ParsedInputValue<'a>> for Option<i64> {
    type Error = ValidationError;

    fn try_from(value: ParsedInputValue<'a>) -> QueryParserResult<Option<i64>> {
        let prisma_value = PrismaValue::try_from(value)?;

        match prisma_value {
            PrismaValue::Int(i) => Ok(Some(i)),
            PrismaValue::Null => Ok(None),
            v => Err(ValidationError::unexpected_runtime_error(format!(
                "Attempted conversion of non-int Prisma value type ({v:?}) into int failed."
            ))),
        }
    }
}

impl<'a> TryFrom<ParsedInputValue<'a>> for bool {
    type Error = ValidationError;

    fn try_from(value: ParsedInputValue<'a>) -> QueryParserResult<bool> {
        let prisma_value = PrismaValue::try_from(value)?;

        match prisma_value {
            PrismaValue::Boolean(b) => Ok(b),
            v => Err(ValidationError::unexpected_runtime_error(format!(
                "Attempted conversion of non-boolean Prisma value type ({v:?}) into bool failed."
            ))),
        }
    }
}

impl<'a> TryFrom<ParsedInputValue<'a>> for RelationLoadStrategy {
    type Error = ValidationError;

    fn try_from(value: ParsedInputValue<'a>) -> QueryParserResult<RelationLoadStrategy> {
        let prisma_value = PrismaValue::try_from(value)?;

        match prisma_value {
            PrismaValue::Enum(e) if e == load_strategy::JOIN => Ok(RelationLoadStrategy::Join),
            PrismaValue::Enum(e) if e == load_strategy::QUERY => Ok(RelationLoadStrategy::Query),
            v => Err(ValidationError::unexpected_runtime_error(format!(
                "Attempted conversion of ParsedInputValue ({v:?}) into relation load strategy enum value failed."
            ))),
        }
    }
}
