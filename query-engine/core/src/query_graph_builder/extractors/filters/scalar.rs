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
    let filter = match filter_key.to_lowercase().as_str() {
        "not" => {
            let inner_object: ParsedInputMap = input.try_into()?;

            let filters = inner_object
                .into_iter()
                .map(|(k, v)| parse(&k, field, v, !reverse))
                .collect::<QueryGraphBuilderResult<Vec<_>>>()?;

            Filter::and(filters)
        }

        "in" => {
            let value: PrismaValue = input.try_into()?;
            match value {
                PrismaValue::Null(_) if reverse => field.not_equals(value),
                PrismaValue::List(values) if reverse => field.is_in(values),

                PrismaValue::Null(_) => field.equals(value),
                PrismaValue::List(values) => field.not_in(values),

                _ => unreachable!(), // Validation guarantees this.
            }
        }

        "equals" if reverse => field.not_equals(as_prisma_value(input)?),
        "contains" if reverse => field.not_contains(as_prisma_value(input)?),
        "starts_with" if reverse => field.not_ends_with(as_prisma_value(input)?),
        "ends_with" if reverse => field.not_ends_with(as_prisma_value(input)?),

        "equals" => field.not_equals(as_prisma_value(input)?),
        "contains" => field.contains(as_prisma_value(input)?),
        "starts_with" => field.starts_with(as_prisma_value(input)?),
        "ends_with" => field.ends_with(as_prisma_value(input)?),

        "lt" if reverse => field.greater_than_or_equals(as_prisma_value(input)?),
        "gt" if reverse => field.less_than_or_equals(as_prisma_value(input)?),
        "lte" if reverse => field.greater_than(as_prisma_value(input)?),
        "gte" if reverse => field.less_than(as_prisma_value(input)?),

        "lt" => field.greater_than_or_equals(as_prisma_value(input)?),
        "gt" => field.less_than_or_equals(as_prisma_value(input)?),
        "lte" => field.greater_than(as_prisma_value(input)?),
        "gte" => field.less_than(as_prisma_value(input)?),

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
