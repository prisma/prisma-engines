use crate::{ParsedInputMap, ParsedInputValue, QueryGraphBuilderError, QueryGraphBuilderResult};
use connector::{ScalarCompare, ScalarFilter};
use prisma_models::{PrismaValue, ScalarFieldRef};
use std::convert::TryInto;

pub fn parse(
    filter_key: &str,
    field: &ScalarFieldRef,
    input: ParsedInputValue,
    reverse: bool,
) -> QueryGraphBuilderResult<Vec<ScalarFilter>> {
    let filter = match filter_key {
        "not" => {
            match input {
                // Support for syntax `{ scalarField: { not: null } }` and `{ scalarField: { not: <value> } }`
                ParsedInputValue::Single(value) => vec![field.not_equals(value)],
                _ => {
                    let inner_object: ParsedInputMap = input.try_into()?;

                    let filters: Vec<ScalarFilter> = inner_object
                        .into_iter()
                        .map(|(k, v)| parse(&k, field, v, !reverse))
                        .collect::<QueryGraphBuilderResult<Vec<Vec<_>>>>()?
                        .into_iter()
                        .flatten()
                        .collect();

                    filters
                }
            }
        }

        "in" => {
            let value: PrismaValue = input.try_into()?;
            vec![match value {
                PrismaValue::Null if reverse => field.not_equals(value),
                PrismaValue::List(values) if reverse => field.not_in(values),

                PrismaValue::Null => field.equals(value),
                PrismaValue::List(values) => field.is_in(values),

                _ => unreachable!(), // Validation guarantees this.
            }]
        }

        // Legacy operation
        "notIn" => {
            let value: PrismaValue = input.try_into()?;
            vec![match value {
                PrismaValue::Null if reverse => field.equals(value), // not not in null => in null
                PrismaValue::List(values) if reverse => field.is_in(values), // not not in values => in values

                PrismaValue::Null => field.not_equals(value),
                PrismaValue::List(values) => field.not_in(values),

                _ => unreachable!(), // Validation guarantees this.
            }]
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

            let filters: Vec<ScalarFilter> = inner_object
                .into_iter()
                .map(|(k, v)| parse(&k, field, v, reverse))
                .collect::<QueryGraphBuilderResult<Vec<Vec<_>>>>()?
                .into_iter()
                .flatten()
                .collect();

            // let map: ParsedInputMap = value.try_into()?;
            // // let mut filters = vec![];
            // for (k, v) in map {
            //     let field = model.fields().find_from_scalar(&key).unwrap();

            //     // filters.extend(extract_scalar_filters(&field, v)?.into_iter().map(||));
            // }


            filters.into_iter().map(|f| )

            todo!()
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
