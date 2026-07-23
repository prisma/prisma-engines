mod composite;
mod filter_fold;
mod filter_grouping;
mod relation;
mod scalar;

use super::utils;
use crate::{
    QueryGraphBuilderError, QueryGraphBuilderResult,
    query_document::{ParsedInputMap, ParsedInputValue},
};
use filter_fold::*;
use filter_grouping::*;
use indexmap::IndexMap;
use query_structure::{prelude::ParentContainer, *};
use schema::constants::filters;
use std::{borrow::Cow, collections::HashMap, convert::TryInto, str::FromStr};

/// Extracts a filter for a unique selector, i.e. a filter that selects exactly one record.
pub fn extract_unique_filter(value_map: ParsedInputMap<'_>, model: &Model) -> QueryGraphBuilderResult<Filter> {
    let tag = value_map.tag.clone();
    // Partition the input into a map containing only the unique fields and one containing all the other filters
    // so that we can parse them separately and ensure we AND both filters
    let (unique_map, rest_map): (IndexMap<_, _>, IndexMap<_, _>) =
        value_map
            .into_iter()
            .partition(|(field_name, _)| match model.fields().find_from_scalar(field_name) {
                Ok(field) => field.unique(),
                Err(_) => utils::resolve_compound_field(field_name, model).is_some(),
            });
    let mut unique_map = ParsedInputMap::from(unique_map);
    let mut rest_map = ParsedInputMap::from(rest_map);
    unique_map.set_tag(tag.clone());
    rest_map.set_tag(tag);

    let unique_filters = internal_extract_unique_filter(unique_map, model)?;
    let rest_filters = extract_filter(rest_map, model)?;

    Ok(Filter::and(vec![unique_filters, rest_filters]))
}

/// Extracts a filter for a unique selector, i.e. a filter that selects exactly one record.
/// The input map must only contain unique & compound unique fields.
fn internal_extract_unique_filter(value_map: ParsedInputMap<'_>, model: &Model) -> QueryGraphBuilderResult<Filter> {
    let filters = value_map
        .into_iter()
        .map(|(field_name, value): (Cow<'_, str>, ParsedInputValue<'_>)| {
            // Always try to resolve regular fields first. If that fails, try to resolve compound fields.
            match model.fields().find_from_scalar(&field_name) {
                Ok(field) => {
                    let value: PrismaValue = value.try_into()?;
                    Ok(field.equals(value))
                }
                Err(_) => utils::resolve_compound_field(&field_name, model)
                    .ok_or_else(|| {
                        QueryGraphBuilderError::AssertionError(format!(
                            "Unable to resolve field {} to a field or set of scalar fields on model {}",
                            field_name,
                            model.name()
                        ))
                    })
                    .and_then(|fields| handle_compound_field(fields, value)),
            }
        })
        .collect::<QueryGraphBuilderResult<Vec<Filter>>>()?;

    Ok(Filter::and(filters))
}

fn handle_compound_field(fields: Vec<ScalarFieldRef>, value: ParsedInputValue<'_>) -> QueryGraphBuilderResult<Filter> {
    let mut input_map: ParsedInputMap<'_> = value.try_into()?;

    let filters: Vec<Filter> = fields
        .into_iter()
        .map(|sf| {
            let pv: PrismaValue = input_map.swap_remove(sf.name()).unwrap().try_into()?;
            Ok(sf.equals(pv))
        })
        .collect::<QueryGraphBuilderResult<Vec<_>>>()?;

    Ok(Filter::And(filters))
}

/// Extracts a regular filter potentially matching many records.
///
/// # Filter rules
///
/// This function recurses to create a structure of `AND/OR/NOT` conditions. The
/// rules are defined as follows:
///
/// | Name | 0 filters         | 1 filter               | n filters            |
/// |---   |---                |---                     |---                   |
/// | OR   | return empty list | validate single filter | validate all filters |
/// | AND  | return all items  | validate single filter | validate all filters |
/// | NOT  | return all items  | validate single filter | validate all filters |
pub fn extract_filter<T>(value_map: ParsedInputMap<'_>, container: T) -> QueryGraphBuilderResult<Filter>
where
    T: Into<ParentContainer>,
{
    let container = container.into();

    // We define an internal function so we can track the recursion depth. Empty
    // filters at the root layer cannot always be removed.
    fn extract_filter(
        value_map: ParsedInputMap<'_>,
        container: &ParentContainer,
        depth: usize,
    ) -> QueryGraphBuilderResult<Filter> {
        let filters = value_map
            .into_iter()
            .map(|(key, value)| {
                // 2 possibilities: Either a filter group (and, or, not) with a vector/object, or a field name with a filter object behind.
                match FilterGrouping::from_str(&key) {
                    Ok(filter_kind) => {
                        let filters = match value {
                            ParsedInputValue::List(values) => values
                                .into_iter()
                                .map(|val| extract_filter(val.try_into()?, container, depth + 1))
                                .collect::<QueryGraphBuilderResult<Vec<Filter>>>()?,

                            // Single map to vec coercion
                            ParsedInputValue::Map(map) => {
                                extract_filter(map, container, depth + 1).map(|res| vec![res])?
                            }

                            _ => unreachable!(),
                        };

                        // strip empty filters
                        let filters = filters
                            .into_iter()
                            .filter(|filter| !matches!(filter, Filter::Empty))
                            .collect::<Vec<Filter>>();

                        match filters.len() {
                            0 => match depth {
                                0 => match filter_kind {
                                    FilterGrouping::And => Ok(Filter::and(filters)),
                                    FilterGrouping::Or => Ok(Filter::or(filters)),
                                    FilterGrouping::Not => Ok(Filter::not(filters)),
                                },
                                _ => Ok(Filter::empty()),
                            },
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
                        let filters = match container.find_field(&key).expect("Invalid field passed validation.") {
                            Field::Relation(rf) => extract_relation_filters(&rf, value),
                            Field::Scalar(sf) => extract_scalar_filters(&sf, value),
                            Field::Composite(cf) => extract_composite_filters(&cf, value),
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

    let filter = extract_filter(value_map, &container, 0)?;
    let filter = merge_search_filters(filter);

    Ok(filter)
}

/// Search filters that have the same query and that are in the same condition block
/// are merged together to optimize the generated SQL statements.
/// This is done in three steps (below transformations are using pseudo-code):
/// 1. We flatten the filter tree.
///    eg: `Filter(And([ScalarFilter, ScalarFilter], And([ScalarFilter])))` -> `Filter(And([ScalarFilter, ScalarFilter, ScalarFilter]))`
/// 2. We index search filters by their query.
///    eg: `Filter(And([SearchFilter("query", [FieldA]), SearchFilter("query", [FieldB])]))` -> `{ "query": [FieldA, FieldB] }`
/// 3. We reconstruct the filter tree and merge the search filters that have the same query along the way
///    eg: `Filter(And([SearchFilter("query", [FieldA]), SearchFilter("query", [FieldB])]))` -> `Filter(And([SearchFilter("query", [FieldA, FieldB])]))`
fn merge_search_filters(filter: Filter) -> Filter {
    // The filter tree _needs_ to be flattened for the merge to work properly
    let flattened = fold_filter(filter);

    match flattened {
        Filter::And(and) => Filter::And(fold_search_filters(&and)),
        Filter::Or(or) => Filter::Or(fold_search_filters(&or)),
        Filter::Not(not) => Filter::Not(fold_search_filters(&not)),
        _ => flattened,
    }
}

fn fold_search_filters(filters: &[Filter]) -> Vec<Filter> {
    let mut filters_by_val: HashMap<PrismaValue, &Filter> = HashMap::new();
    let mut projections_by_val: HashMap<PrismaValue, Vec<ScalarProjection>> = HashMap::new();
    let mut output: Vec<Filter> = vec![];

    // Gather search filters that have the same condition
    for filter in filters.iter() {
        match filter {
            Filter::Scalar(sf) => match sf.condition {
                ScalarCondition::Search(ref pv, _) => {
                    let pv = pv.as_value().unwrap();
                    // If there's already an entry, then store the "additional" projections that will need to be merged
                    if let Some(projections) = projections_by_val.get_mut(pv) {
                        projections.push(sf.projection.clone());
                    } else {
                        // Otherwise, store the first search filter found on which we'll merge the additional projections
                        projections_by_val.insert(pv.clone(), vec![]);
                        filters_by_val.insert(pv.clone(), filter);
                    }
                }
                _ => output.push(filter.clone()),
            },
            Filter::And(and) => {
                output.push(Filter::And(fold_search_filters(and)));
            }
            Filter::Or(or) => {
                output.push(Filter::Or(fold_search_filters(or)));
            }
            Filter::Not(not) => {
                output.push(Filter::Not(fold_search_filters(not)));
            }
            x => output.push(x.clone()),
        }
    }

    // Merge the search filters that have the same condition
    for (pv, filter) in filters_by_val.into_iter() {
        let projections = projections_by_val.get_mut(&pv).unwrap();
        let mut filter = filter.clone();

        match filter {
            Filter::Scalar(ref mut sf) => match sf.condition {
                ScalarCondition::Search(_, ref mut search_proj) => {
                    search_proj.append(projections);
                }
                ScalarCondition::NotSearch(_, ref mut search_proj) => {
                    search_proj.append(projections);
                }
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }

        output.push(filter.clone());
    }

    output
}

/// Field is the field the filter is refering to and `value` is the passed filter. E.g. `where: { <field>: <value> }.
/// `value` can be either a flat scalar (for shorthand filter notation) or an object (full filter syntax).
fn extract_scalar_filters(field: &ScalarFieldRef, value: ParsedInputValue<'_>) -> QueryGraphBuilderResult<Vec<Filter>> {
    match value {
        ParsedInputValue::Single(pv) => Ok(vec![field.equals(pv)]),
        ParsedInputValue::Map(mut filter_map) => {
            let mode = match filter_map.swap_remove(filters::MODE) {
                Some(i) => parse_query_mode(i)?,
                None => QueryMode::Default,
            };

            let mut filters: Vec<Filter> = scalar::ScalarFilterParser::new(field, false).parse(filter_map)?;

            filters.iter_mut().for_each(|f| f.set_mode(mode.clone()));
            Ok(filters)
        }
        x => Err(QueryGraphBuilderError::InputError(format!(
            "Invalid scalar filter input: {x:?}"
        ))),
    }
}

/// Field is the field the filter is refering to and `value` is the passed filter. E.g. `where: { <field>: <value> }.
/// `value` can be either a filter object (for shorthand filter notation) or an object (full filter syntax).
fn extract_relation_filters(
    field: &RelationFieldRef,
    value: ParsedInputValue<'_>,
) -> QueryGraphBuilderResult<Vec<Filter>> {
    match value {
        // Implicit is null filter (`where: { <field>: null }`)
        ParsedInputValue::Single(PrismaValue::Null) => Ok(vec![field.one_relation_is_null()]),

        // Complex relation filter
        ParsedInputValue::Map(filter_map) if filter_map.is_relation_envelope() => filter_map
            .into_iter()
            .map(|(k, v)| relation::parse(&k, field, v))
            .collect::<QueryGraphBuilderResult<Vec<_>>>(),

        // Implicit is
        ParsedInputValue::Map(filter_map) => {
            extract_filter(filter_map, field.related_model()).map(|filter| vec![field.to_one_related(filter)])
        }

        x => Err(QueryGraphBuilderError::InputError(format!(
            "Invalid relation filter input: {x:?}"
        ))),
    }
}

fn parse_query_mode(input: ParsedInputValue<'_>) -> QueryGraphBuilderResult<QueryMode> {
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

/// Field is the field the filter is refering to and `value` is the passed filter. E.g. `where: { <field>: <value> }.
/// `value` can be either a flat scalar (for shorthand filter notation) or an object (full filter syntax).
fn extract_composite_filters(
    field: &CompositeFieldRef,
    value: ParsedInputValue<'_>,
) -> QueryGraphBuilderResult<Vec<Filter>> {
    match value {
        ParsedInputValue::Single(val) => Ok(vec![field.equals(val)]), // Todo: Do we want to do coercions here? (list, object)
        ParsedInputValue::List(_) => Ok(vec![field.equals(PrismaValue::List(value.try_into()?))]),
        ParsedInputValue::Map(filter_map) => Ok(vec![composite::parse(filter_map, field, false)?]),
        x => Err(QueryGraphBuilderError::InputError(format!(
            "Invalid composite filter input: {x:?}"
        ))),
    }
}
