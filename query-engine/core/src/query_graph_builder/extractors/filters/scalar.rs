use crate::{ParsedInputMap, ParsedInputValue, QueryGraphBuilderError, QueryGraphBuilderResult};
use connector::{Filter, ScalarCompare};
use prisma_models::{PrismaValue, ScalarFieldRef};
use std::convert::TryInto;

pub fn parse(
    filter_key: &str,
    field: &ScalarFieldRef,
    input: ParsedInputValue,
    reverse: bool,
) -> QueryGraphBuilderResult<Vec<Filter>> {
    let filter = match filter_key {
        "not" => {
            match input {
                // Support for syntax `{ scalarField: { not: null } }` and `{ scalarField: { not: <value> } }`
                ParsedInputValue::Single(value) => {
                    vec![field.not_equals(value)]
                }
                _ => {
                    let inner_object: ParsedInputMap = input.try_into()?;

                    inner_object
                        .into_iter()
                        .map(|(k, v)| parse(&k, field, v, !reverse))
                        .collect::<QueryGraphBuilderResult<Vec<_>>>()?
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>()
                }
            }
        }

        "in" => {
            let value: PrismaValue = input.try_into()?;
            let filter = match value {
                PrismaValue::Null if reverse => field.not_equals(value),
                PrismaValue::List(values) if reverse => field.not_in(values),

                PrismaValue::Null => field.equals(value),
                PrismaValue::List(values) => field.is_in(values),

                _ => unreachable!(), // Validation guarantees this.
            };

            vec![filter]
        }

        // Legacy operation
        "notIn" => {
            let value: PrismaValue = input.try_into()?;
            let filter = match value {
                PrismaValue::Null if reverse => field.equals(value), // not not in null => in null
                PrismaValue::List(values) if reverse => field.is_in(values), // not not in values => in values

                PrismaValue::Null => field.not_equals(value),
                PrismaValue::List(values) => field.not_in(values),

                _ => unreachable!(), // Validation guarantees this.
            };

            vec![filter]
        }

        "equals" if reverse => vec![field.not_equals(as_prisma_value(input)?)],
        "contains" if reverse => vec![field.not_contains(as_prisma_value(input)?)],
        "startsWith" if reverse => vec![field.not_starts_with(as_prisma_value(input)?)],
        "endsWith" if reverse => vec![field.not_ends_with(as_prisma_value(input)?)],

        "equals" => vec![field.equals(as_prisma_value(input)?)],
        "contains" => vec![field.contains(as_prisma_value(input)?)],
        "startsWith" => vec![field.starts_with(as_prisma_value(input)?)],
        "endsWith" => vec![field.ends_with(as_prisma_value(input)?)],

        "lt" if reverse => vec![field.greater_than_or_equals(as_prisma_value(input)?)],
        "gt" if reverse => vec![field.less_than_or_equals(as_prisma_value(input)?)],
        "lte" if reverse => vec![field.greater_than(as_prisma_value(input)?)],
        "gte" if reverse => vec![field.less_than(as_prisma_value(input)?)],

        "lt" => vec![field.less_than(as_prisma_value(input)?)],
        "gt" => vec![field.greater_than(as_prisma_value(input)?)],
        "lte" => vec![field.less_than_or_equals(as_prisma_value(input)?)],
        "gte" => vec![field.greater_than_or_equals(as_prisma_value(input)?)],

        // Aggregation filters
        "count" => todo!(),
        "avg" => {
            let inner_object: ParsedInputMap = input.try_into()?;
            let mut results = vec![];

            for (k, v) in inner_object {
                // let filters = super::extract_scalar_filters(field, v)?;
                let filters = parse(&k, field, v, reverse)?;
                results.extend(filters);
            }

            results.into_iter().map(|f| Filter::average(f)).collect()
        }
        "sum" => todo!(),
        "min" => todo!(),
        "max" => todo!(),

        _ => {
            return Err(QueryGraphBuilderError::InputError(format!(
                "{} is not a valid scalar filter operation",
                filter_key
            )))
        }
    };

    Ok(filter)
}

fn as_prisma_value(input: ParsedInputValue) -> QueryGraphBuilderResult<PrismaValue> {
    Ok(input.try_into()?)
}
