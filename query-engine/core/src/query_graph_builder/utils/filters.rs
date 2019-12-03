use super::*;
use crate::{
    query_document::{ParsedInputMap, ParsedInputValue},
    schema_builder::compound_field_name,
};
use connector::{filter::Filter, RelationCompare, ScalarCompare};
use prisma_models::{Field, ModelRef, PrismaListValue, PrismaValue, RelationFieldRef, ScalarFieldRef};
use std::{collections::BTreeMap, convert::TryFrom, convert::TryInto};

lazy_static! {
    /// Filter operations in descending order of how they should be checked.
    static ref FILTER_OPERATIONS: Vec<FilterOp> = vec![
        FilterOp::NotIn,
        FilterOp::NotContains,
        FilterOp::NotStartsWith,
        FilterOp::NotEndsWith,
        FilterOp::In,
        FilterOp::Not,
        FilterOp::Lt,
        FilterOp::Lte,
        FilterOp::Gt,
        FilterOp::Gte,
        FilterOp::Contains,
        FilterOp::StartsWith,
        FilterOp::EndsWith,
        FilterOp::Some,
        FilterOp::None,
        FilterOp::Every,
        FilterOp::NestedAnd,
        FilterOp::NestedOr,
        FilterOp::NestedNot,
        FilterOp::Field, // Needs to be last
    ];
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum FilterOp {
    In,
    NotIn,
    Not,
    Lt,
    Lte,
    Gt,
    Gte,
    Contains,
    NotContains,
    StartsWith,
    NotStartsWith,
    EndsWith,
    NotEndsWith,
    Some,
    None,
    Every,
    NestedAnd,
    NestedOr,
    NestedNot,
    Field,
}

impl FilterOp {
    pub fn find_op(name: &str) -> Option<FilterOp> {
        FILTER_OPERATIONS
            .iter()
            .find(|op| {
                let op_suffix: &'static str = op.suffix();
                name.ends_with(op_suffix)
            })
            .copied()
    }

    pub fn suffix(self) -> &'static str {
        match self {
            FilterOp::In => "_in",
            FilterOp::NotIn => "_not_in",
            FilterOp::Not => "_not",
            FilterOp::Lt => "_lt",
            FilterOp::Lte => "_lte",
            FilterOp::Gt => "_gt",
            FilterOp::Gte => "_gte",
            FilterOp::Contains => "_contains",
            FilterOp::NotContains => "_not_contains",
            FilterOp::StartsWith => "_starts_with",
            FilterOp::NotStartsWith => "_not_starts_with",
            FilterOp::EndsWith => "_ends_with",
            FilterOp::NotEndsWith => "_not_ends_with",
            FilterOp::Some => "_some",
            FilterOp::None => "_none",
            FilterOp::Every => "_every",
            FilterOp::NestedAnd => "AND",
            FilterOp::NestedOr => "OR",
            FilterOp::NestedNot => "NOT",
            FilterOp::Field => "",
        }
    }
}

pub fn extract_filter(
    value_map: BTreeMap<String, ParsedInputValue>,
    model: &ModelRef,
    match_suffix: bool,
) -> QueryGraphBuilderResult<Filter> {
    let filters = value_map
        .into_iter()
        .map(|(key, value): (String, ParsedInputValue)| {
            let op = if match_suffix {
                FilterOp::find_op(key.as_str()).unwrap()
            } else {
                FilterOp::Field
            };

            match op {
                op if (op == FilterOp::NestedAnd || op == FilterOp::NestedOr || op == FilterOp::NestedNot) => {
                    let value: QueryGraphBuilderResult<Vec<Filter>> = match value {
                        ParsedInputValue::List(values) => values
                            .into_iter()
                            .map(|val| extract_filter(val.try_into()?, model, match_suffix))
                            .collect(),

                        ParsedInputValue::Map(map) => extract_filter(map, model, match_suffix).map(|res| vec![res]),

                        _ => unreachable!(),
                    };

                    value.map(|value| match op {
                        FilterOp::NestedAnd => Filter::and(value),
                        FilterOp::NestedOr => Filter::or(value),
                        FilterOp::NestedNot => Filter::not(value),
                        _ => unreachable!(),
                    })
                }
                op => {
                    let op_name: &'static str = op.suffix();
                    let field_name = key.trim_end_matches(op_name);

                    // Always try to resolve regular fields first. If that fails, try to resolve compound fields.
                    match model.fields().find_from_all(&field_name) {
                        Ok(field) => match field {
                            Field::Scalar(field) => handle_scalar_field(field, value, &op),
                            Field::Relation(field) => handle_relation_field(field, value, &op, match_suffix),
                        },
                        Err(_) => find_index_fields(&field_name, &model)
                            .and_then(|fields| handle_compound_field(fields, value))
                            .map_err(|_| {
                                QueryGraphBuilderError::AssertionError(format!(
                                    "Unable to resolve field {} to a field or index on model {}",
                                    field_name, model.name
                                ))
                            }),
                    }
                }
            }
        })
        .collect::<QueryGraphBuilderResult<Vec<Filter>>>()?;

    Ok(Filter::and(filters))
}

fn handle_scalar_field(
    field: &ScalarFieldRef,
    value: ParsedInputValue,
    op: &FilterOp,
) -> QueryGraphBuilderResult<Filter> {
    let value: PrismaValue = value.try_into()?;
    Ok(match op {
        FilterOp::In => field.is_in(PrismaListValue::try_from(value)?),
        FilterOp::NotIn => field.not_in(PrismaListValue::try_from(value)?),
        FilterOp::Not => field.not_equals(value),
        FilterOp::Lt => field.less_than(value),
        FilterOp::Lte => field.less_than_or_equals(value),
        FilterOp::Gt => field.greater_than(value),
        FilterOp::Gte => field.greater_than_or_equals(value),
        FilterOp::Contains => field.contains(value),
        FilterOp::NotContains => field.not_contains(value),
        FilterOp::StartsWith => field.starts_with(value),
        FilterOp::NotStartsWith => field.not_starts_with(value),
        FilterOp::EndsWith => field.ends_with(value),
        FilterOp::NotEndsWith => field.not_ends_with(value),
        FilterOp::Field => field.equals(value),
        _ => unreachable!(),
    })
}

fn handle_relation_field(
    field: &RelationFieldRef,
    value: ParsedInputValue,
    op: &FilterOp,
    match_suffix: bool,
) -> QueryGraphBuilderResult<Filter> {
    let value: Option<BTreeMap<String, ParsedInputValue>> = value.try_into()?;

    Ok(match (op, value) {
        (FilterOp::Some, Some(value)) => {
            field.at_least_one_related(extract_filter(value, &field.related_model(), match_suffix)?)
        }
        (FilterOp::None, Some(value)) => field.no_related(extract_filter(value, &field.related_model(), match_suffix)?),
        (FilterOp::Every, Some(value)) => {
            field.every_related(extract_filter(value, &field.related_model(), match_suffix)?)
        }
        (FilterOp::Field, Some(value)) => {
            field.to_one_related(extract_filter(value, &field.related_model(), match_suffix)?)
        }
        (FilterOp::Field, None) => field.one_relation_is_null(),
        _ => unreachable!(),
    })
}

fn handle_compound_field(fields: Vec<ScalarFieldRef>, value: ParsedInputValue) -> QueryGraphBuilderResult<Filter> {
    let mut value: ParsedInputMap = value.try_into()?;

    let filters: Vec<Filter> = fields
        .into_iter()
        .map(|field| {
            let value: PrismaValue = value.remove(&field.name).unwrap().try_into()?;
            Ok(field.equals(value))
        })
        .collect::<QueryGraphBuilderResult<Vec<_>>>()?;

    Ok(Filter::And(filters))
}

/// Attempts to match a given name to the (schema) name of a compound indexes on the model and returns the first match.
fn find_index_fields(name: &str, model: &ModelRef) -> QueryGraphBuilderResult<Vec<ScalarFieldRef>> {
    model
        .unique_indexes()
        .into_iter()
        .find(|index| &compound_field_name(index) == name)
        .map(|index| index.fields())
        .ok_or(QueryGraphBuilderError::AssertionError(format!(
            "Unable to resolve {} to an index on model {}",
            name, model.name
        )))
}
