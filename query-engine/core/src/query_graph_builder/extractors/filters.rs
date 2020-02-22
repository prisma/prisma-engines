use super::utils;
use crate::{
    query_document::{ParsedInputMap, ParsedInputValue},
    QueryGraphBuilderError, QueryGraphBuilderResult,
};
use connector::{filter::Filter, RelationCompare, ScalarCompare};
use prisma_models::{Field, ModelRef, PrismaValue, RelationFieldRef, ScalarFieldRef};
use std::{collections::BTreeMap, convert::TryInto};

static FILTER_OPERATIONS: &'static [FilterOp] = &[
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
                        Err(_) => utils::resolve_compound_field(&field_name, &model)
                            .ok_or(QueryGraphBuilderError::AssertionError(format!(
                                "Unable to resolve field {} to a field or set of fields on model {}",
                                field_name, model.name
                            )))
                            .and_then(|fields| handle_compound_field(fields, value)),
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
    let dsf = field.data_source_field();

    Ok(match (op, value) {
        (FilterOp::In, PrismaValue::Null) => dsf.equals(PrismaValue::Null),
        (FilterOp::In, PrismaValue::List(values)) => dsf.is_in(values),
        (FilterOp::NotIn, PrismaValue::Null) => dsf.not_equals(PrismaValue::Null),
        (FilterOp::NotIn, PrismaValue::List(values)) => dsf.not_in(values),
        (FilterOp::Not, val) => dsf.not_equals(val),
        (FilterOp::Lt, val) => dsf.less_than(val),
        (FilterOp::Lte, val) => dsf.less_than_or_equals(val),
        (FilterOp::Gt, val) => dsf.greater_than(val),
        (FilterOp::Gte, val) => dsf.greater_than_or_equals(val),
        (FilterOp::Contains, val) => dsf.contains(val),
        (FilterOp::NotContains, val) => dsf.not_contains(val),
        (FilterOp::StartsWith, val) => dsf.starts_with(val),
        (FilterOp::NotStartsWith, val) => dsf.not_starts_with(val),
        (FilterOp::EndsWith, val) => dsf.ends_with(val),
        (FilterOp::NotEndsWith, val) => dsf.not_ends_with(val),
        (FilterOp::Field, val) => dsf.equals(val),
        (_, _) => unreachable!(),
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
            Ok(field.data_source_field().equals(value))
        })
        .collect::<QueryGraphBuilderResult<Vec<_>>>()?;

    Ok(Filter::And(filters))
}
