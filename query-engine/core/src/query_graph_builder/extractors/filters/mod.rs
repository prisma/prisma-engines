mod filter_grouping;
mod relation;
mod scalar;

use super::utils;
use crate::{
    query_document::{ParsedInputMap, ParsedInputValue},
    QueryGraphBuilderError, QueryGraphBuilderResult,
};
use connector::{filter::Filter, QueryMode, RelationCompare, ScalarCompare};
use filter_grouping::*;
use prisma_models::{Field, ModelRef, PrismaValue, RelationFieldRef, ScalarFieldRef};
use std::{convert::TryInto, str::FromStr};

/// Extracts a filter for a unique selector, i.e. a filter that selects exactly one record.
pub fn extract_unique_filter(value_map: ParsedInputMap, model: &ModelRef) -> QueryGraphBuilderResult<Filter> {
    let filters = value_map
        .into_iter()
        .map(|(field_name, value): (String, ParsedInputValue)| {
            // Always try to resolve regular fields first. If that fails, try to resolve compound fields.
            match model.fields().find_from_scalar(&field_name) {
                Ok(field) => {
                    let value: PrismaValue = value.try_into()?;
                    Ok(field.equals(value))
                }
                Err(_) => utils::resolve_compound_field(&field_name, &model)
                    .ok_or(QueryGraphBuilderError::AssertionError(format!(
                        "Unable to resolve field {} to a field or set of scalar fields on model {}",
                        field_name, model.name
                    )))
                    .and_then(|fields| handle_compound_field(fields, value)),
            }
        })
        .collect::<QueryGraphBuilderResult<Vec<Filter>>>()?;

    Ok(Filter::and(filters))
}

fn handle_compound_field(fields: Vec<ScalarFieldRef>, value: ParsedInputValue) -> QueryGraphBuilderResult<Filter> {
    let mut input_map: ParsedInputMap = value.try_into()?;

    let filters: Vec<Filter> = fields
        .into_iter()
        .map(|sf| {
            let pv: PrismaValue = input_map.remove(&sf.name).unwrap().try_into()?;
            Ok(sf.equals(pv))
        })
        .collect::<QueryGraphBuilderResult<Vec<_>>>()?;

    Ok(Filter::And(filters))
}

/// Extracts a regular filter potentially matching many records.
///
/// This function recurses to create a structure of `AND/OR/NOT` conditions.
/// After each recursion we check whether the filter group was populated. If no
/// filter was defined a `Filter::Empty` is returned. If an `AND/OR` filter group
/// was found that has only a single member, we only return the inner filter.
/// This results in an optimal filter tree.
pub fn extract_filter(value_map: ParsedInputMap, model: &ModelRef) -> QueryGraphBuilderResult<Filter> {
    let filters = value_map
        .into_iter()
        .map(|(key, value)| {
            // 2 possibilities: Either a filter group (and, or, not) with a vector/object, or a field name with a filter object behind.
            match FilterGrouping::from_str(&key) {
                Ok(filter_kind) => {
                    let filters = match value {
                        ParsedInputValue::List(values) => values
                            .into_iter()
                            .map(|val| extract_filter(val.try_into()?, model))
                            .collect::<QueryGraphBuilderResult<Vec<Filter>>>()?,

                        // Single map to vec coercion
                        ParsedInputValue::Map(map) => extract_filter(map, model).map(|res| vec![res])?,

                        _ => unreachable!(),
                    };

                    // strip empty filters
                    let filters = filters
                        .into_iter()
                        .filter(|filter| !matches!(filter, Filter::Empty))
                        .collect::<Vec<Filter>>();

                    match filters.len() {
                        0 => Ok(Filter::empty()),
                        1 => match filter_kind {
                            FilterGrouping::Not => Ok(Filter::not(filters)),
                            _ => Ok(filters.into_iter().next().unwrap()),
                        },
                        _ => match filter_kind {
                            FilterGrouping::And => Ok(Filter::and(filters)),
                            FilterGrouping::Or => Ok(Filter::or(filters)),
                            FilterGrouping::Not => Ok(Filter::not(filters)),
                        },
                    }
                }
                Err(_) => {
                    let filters = match model.fields().find_from_all(&key)? {
                        Field::Relation(rf) => extract_relation_filters(rf, value),
                        Field::Scalar(sf) => extract_scalar_filters(sf, value),
                    }?;

                    // strip empty filters
                    let filters = filters
                        .into_iter()
                        .filter(|filter| !matches!(filter, Filter::Empty))
                        .collect::<Vec<Filter>>();

                    match filters.len() {
                        0 => Ok(Filter::empty()),
                        1 => Ok(filters.into_iter().next().unwrap()),
                        _ => Ok(Filter::and(filters)),
                    }
                }
            }
        })
        .filter(|filter| !matches!(filter, Ok(Filter::Empty)))
        .collect::<QueryGraphBuilderResult<Vec<Filter>>>()?;

    match filters.len() {
        0 => Ok(Filter::empty()),
        1 => Ok(filters.into_iter().next().unwrap()),
        _ => Ok(Filter::and(filters)),
    }
}

/// Field is the field the filter is refering to and `value` is the passed filter. E.g. `where: { <field>: <value> }.
/// `value` can be either a flat scalar (for shorthand filter notation) or an object (full filter syntax).
fn extract_scalar_filters(field: &ScalarFieldRef, value: ParsedInputValue) -> QueryGraphBuilderResult<Vec<Filter>> {
    match value {
        ParsedInputValue::Single(pv) => Ok(vec![field.equals(pv)]),
        ParsedInputValue::Map(mut filter_map) => {
            let mode = match filter_map.remove("mode") {
                Some(i) => parse_query_mode(i)?,
                None => QueryMode::Default,
            };

            let mut filters = filter_map
                .into_iter()
                .map(|(k, v)| scalar::parse(&k, field, v, false))
                .collect::<QueryGraphBuilderResult<Vec<_>>>()?;

            filters.iter_mut().for_each(|f| f.set_mode(mode.clone()));

            Ok(filters)
        }
        x => Err(QueryGraphBuilderError::InputError(format!(
            "Invalid scalar filter input: {:?}",
            x
        ))),
    }
}

/// Field is the field the filter is refering to and `value` is the passed filter. E.g. `where: { <field>: <value> }.
/// `value` can be either a filter object (for shorthand filter notation) or an object (full filter syntax).
fn extract_relation_filters(field: &RelationFieldRef, value: ParsedInputValue) -> QueryGraphBuilderResult<Vec<Filter>> {
    match value {
        // Implicit is null filter (`where: { <field>: null }`)
        ParsedInputValue::Single(PrismaValue::Null) => Ok(vec![field.one_relation_is_null()]),

        // Either implicit `is`, or complex filter.
        ParsedInputValue::Map(filter_map) => {
            // Two options: An intermediate object with `is`, `every`, etc., or directly the object to filter with implicit `is`.
            // There are corner cases where it is unclear what object should be used, due to overlap of fields of the related model and the filter object.
            // The parse heuristic is to first try to parse the map as the full filter object and if that fails, parse it as implicit `is` filter instead.
            filter_map
                .clone()
                .into_iter()
                .map(|(k, v)| relation::parse(&k, field, v))
                .collect::<QueryGraphBuilderResult<Vec<_>>>()
                .or_else(|_| {
                    extract_filter(filter_map, &field.related_model()).map(|filter| vec![field.to_one_related(filter)])
                })
        }

        x => Err(QueryGraphBuilderError::InputError(format!(
            "Invalid relation filter input: {:?}",
            x
        ))),
    }
}

fn parse_query_mode(input: ParsedInputValue) -> QueryGraphBuilderResult<QueryMode> {
    let value: PrismaValue = input.try_into()?;
    let s = match value {
        PrismaValue::Enum(s) => s,
        PrismaValue::String(s) => s,
        _ => unreachable!(),
    };

    Ok(match s.as_str() {
        "default" => QueryMode::Default,
        "insensitive" => QueryMode::Insensitive,
        _ => unreachable!(),
    })
}
