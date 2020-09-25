use crate::{ParsedInputMap, ParsedInputValue, QueryGraphBuilderError, QueryGraphBuilderResult};
use connector::{Filter, ScalarCompare};
use prisma_models::{PrismaValue, ScalarFieldRef};
use std::convert::TryInto;

pub fn parse(
    filter_key: &str,
    field: &ScalarFieldRef,
    input: ParsedInputValue,
    reverse: bool,
) -> QueryGraphBuilderResult<Filter> {
    let filter = match filter_key {
        "not" => {
            match input {
                // Support for syntax `{ scalarField: { not: null } }` and `{ scalarField: { not: <value> } }`
                ParsedInputValue::Single(value) => field.not_equals(value),
                _ => {
                    let inner_object: ParsedInputMap = input.try_into()?;

                    let filters = inner_object
                        .into_iter()
                        .map(|(k, v)| parse(&k, field, v, !reverse))
                        .collect::<QueryGraphBuilderResult<Vec<_>>>()?;

                    Filter::and(filters)
                }
            }
        }

        "in" => {
            let value: PrismaValue = input.try_into()?;
            match value {
                PrismaValue::Null if reverse => field.not_equals(value),
                PrismaValue::List(values) if reverse => field.not_in(values),

                PrismaValue::Null => field.equals(value),
                PrismaValue::List(values) => field.is_in(values),

                _ => unreachable!(), // Validation guarantees this.
            }
        }

        "notIn" => {
            // Legacy operation
            let value: PrismaValue = input.try_into()?;
            match value {
                PrismaValue::Null if reverse => field.equals(value), // not not in null => in null
                PrismaValue::List(values) if reverse => field.is_in(values), // not not in values => in values

                PrismaValue::Null => field.not_equals(value),
                PrismaValue::List(values) => field.not_in(values),

                _ => unreachable!(), // Validation guarantees this.
            }
        }

        "equals" if reverse => field.not_equals(as_prisma_value(input)?),
        "contains" if reverse => field.not_contains(as_prisma_value(input)?),
        "startsWith" if reverse => field.not_starts_with(as_prisma_value(input)?),
        "endsWith" if reverse => field.not_ends_with(as_prisma_value(input)?),

        "equals" => field.equals(as_prisma_value(input)?),
        "contains" => field.contains(as_prisma_value(input)?),
        "startsWith" => field.starts_with(as_prisma_value(input)?),
        "endsWith" => field.ends_with(as_prisma_value(input)?),

        "lt" if reverse => field.greater_than_or_equals(as_prisma_value(input)?),
        "gt" if reverse => field.less_than_or_equals(as_prisma_value(input)?),
        "lte" if reverse => field.greater_than(as_prisma_value(input)?),
        "gte" if reverse => field.less_than(as_prisma_value(input)?),

        "lt" => field.less_than(as_prisma_value(input)?),
        "gt" => field.greater_than(as_prisma_value(input)?),
        "lte" => field.less_than_or_equals(as_prisma_value(input)?),
        "gte" => field.greater_than_or_equals(as_prisma_value(input)?),

        _ => Err(QueryGraphBuilderError::InputError(format!(
            "{} is not a valid scalar filter operation",
            filter_key
        )))?,
    };

    Ok(filter)
}

fn as_prisma_value(input: ParsedInputValue) -> QueryGraphBuilderResult<PrismaValue> {
    Ok(input.try_into()?)
}
