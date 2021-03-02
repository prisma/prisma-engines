use crate::{
    constants::inputs::filters, ParsedInputMap, ParsedInputValue, QueryGraphBuilderError, QueryGraphBuilderResult,
};
use connector::{Filter, ScalarCompare, ScalarListCompare};
use prisma_models::{PrismaValue, ScalarFieldRef};
use std::convert::TryInto;

#[tracing::instrument(name = "parse_scalar_field", skip(filter_key, field, input, reverse))]
pub fn parse(
    filter_key: &str,
    field: &ScalarFieldRef,
    input: ParsedInputValue,
    reverse: bool,
) -> QueryGraphBuilderResult<Vec<Filter>> {
    let filter = match filter_key {
        filters::NOT_LOWERCASE => {
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

        filters::IN => {
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
        filters::NOT_IN => {
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

        filters::EQUALS if reverse => vec![field.not_equals(as_prisma_value(input)?)],
        filters::CONTAINS if reverse => vec![field.not_contains(as_prisma_value(input)?)],
        filters::STARTS_WITH if reverse => vec![field.not_starts_with(as_prisma_value(input)?)],
        filters::ENDS_WITH if reverse => vec![field.not_ends_with(as_prisma_value(input)?)],

        filters::EQUALS => vec![field.equals(as_prisma_value(input)?)],
        filters::CONTAINS => vec![field.contains(as_prisma_value(input)?)],
        filters::STARTS_WITH => vec![field.starts_with(as_prisma_value(input)?)],
        filters::ENDS_WITH => vec![field.ends_with(as_prisma_value(input)?)],

        filters::LOWER_THAN if reverse => vec![field.greater_than_or_equals(as_prisma_value(input)?)],
        filters::GREATER_THAN if reverse => vec![field.less_than_or_equals(as_prisma_value(input)?)],
        filters::LOWER_THAN_OR_EQUAL if reverse => vec![field.greater_than(as_prisma_value(input)?)],
        filters::GREATER_THAN_OR_EQUAL if reverse => vec![field.less_than(as_prisma_value(input)?)],

        filters::LOWER_THAN => vec![field.less_than(as_prisma_value(input)?)],
        filters::GREATER_THAN => vec![field.greater_than(as_prisma_value(input)?)],
        filters::LOWER_THAN_OR_EQUAL => vec![field.less_than_or_equals(as_prisma_value(input)?)],
        filters::GREATER_THAN_OR_EQUAL => vec![field.greater_than_or_equals(as_prisma_value(input)?)],

        // List-specific filters
        filters::HAS => vec![field.contains_element(as_prisma_value(input)?)],
        filters::HAS_EVERY => vec![field.contains_every_element(as_prisma_value_list(input)?)],
        filters::HAS_SOME => vec![field.contains_some_element(as_prisma_value_list(input)?)],
        filters::IS_EMPTY => vec![field.is_empty_list(input.try_into()?)],

        // Aggregation filters
        filters::COUNT => aggregation_filter(field, input, reverse, Filter::count)?,
        filters::AVG => aggregation_filter(field, input, reverse, Filter::average)?,
        filters::SUM => aggregation_filter(field, input, reverse, Filter::sum)?,
        filters::MIN => aggregation_filter(field, input, reverse, Filter::min)?,
        filters::MAX => aggregation_filter(field, input, reverse, Filter::max)?,

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

fn as_prisma_value_list(input: ParsedInputValue) -> QueryGraphBuilderResult<Vec<PrismaValue>> {
    Ok(input.try_into()?)
}

fn aggregation_filter<F>(
    field: &ScalarFieldRef,
    input: ParsedInputValue,
    reverse: bool,
    func: F,
) -> QueryGraphBuilderResult<Vec<Filter>>
where
    F: Fn(Filter) -> Filter,
{
    let inner_object: ParsedInputMap = input.try_into()?;
    let mut results = vec![];

    for (k, v) in inner_object {
        let filters = parse(&k, field, v, reverse)?;
        results.extend(filters);
    }

    Ok(results.into_iter().map(|filter| func(filter)).collect())
}
