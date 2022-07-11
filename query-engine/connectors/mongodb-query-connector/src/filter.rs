use crate::{error::MongoError, join::JoinStage, IntoBson};
use connector_interface::{
    AggregationFilter, CompositeCondition, CompositeFilter, Filter, OneRelationIsNullFilter, QueryMode, RelationFilter,
    ScalarCompare, ScalarCondition, ScalarFilter, ScalarListFilter, ScalarProjection,
};
use mongodb::bson::{doc, Bson, Document};
use prisma_models::{CompositeFieldRef, PrismaValue, ScalarFieldRef, TypeIdentifier};

#[derive(Debug, Clone)]
pub(crate) enum MongoFilter {
    Scalar(Document),
    Composite(Document),
    Relation(MongoRelationFilter),
}

impl MongoFilter {
    pub(crate) fn render(self) -> (Document, Vec<JoinStage>) {
        match self {
            Self::Scalar(document) => (document, vec![]),
            Self::Composite(document) => (document, vec![]),
            Self::Relation(rf) => (rf.filter, rf.joins),
        }
    }

    pub(crate) fn relation(filter: Document, joins: Vec<JoinStage>) -> Self {
        Self::Relation(MongoRelationFilter { filter, joins })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MongoRelationFilter {
    /// The filter that has to be applied to this layer of nesting (after all joins on this layer are done).
    pub filter: Document,

    /// All join trees required on this level to make the above filter work.
    pub joins: Vec<JoinStage>, // todo this is confusing, because in the "merged" state this will always be len = 1.
}

/// Builds a MongoDB query filter from a Prisma filter.
pub(crate) fn convert_filter(
    filter: Filter,
    invert: bool,
    prefix: impl Into<FilterPrefix>,
) -> crate::Result<MongoFilter> {
    convert_filter_internal(filter, invert, false, prefix)
}

fn convert_filter_internal(
    filter: Filter,
    invert: bool,
    invert_undefined_exclusion: bool,
    prefix: impl Into<FilterPrefix>,
) -> crate::Result<MongoFilter> {
    let prefix = prefix.into();
    let filter = fold_compounds(filter);

    let filter_pair = match filter {
        Filter::And(filters) if invert => {
            coerce_empty(false, "$or", filters, invert, invert_undefined_exclusion, prefix)?
        }
        Filter::And(filters) => coerce_empty(true, "$and", filters, invert, invert_undefined_exclusion, prefix)?,

        Filter::Or(filters) if invert => {
            coerce_empty(true, "$and", filters, invert, invert_undefined_exclusion, prefix)?
        }
        Filter::Or(filters) => coerce_empty(false, "$or", filters, invert, invert_undefined_exclusion, prefix)?,

        Filter::Not(filters) if invert => {
            coerce_empty(false, "$or", filters, !invert, invert_undefined_exclusion, prefix)?
        }
        Filter::Not(filters) => coerce_empty(true, "$and", filters, !invert, invert_undefined_exclusion, prefix)?,

        Filter::Scalar(sf) => scalar_filter(sf, invert, invert_undefined_exclusion, false, prefix)?,
        Filter::Empty => MongoFilter::Scalar(doc! {}),
        Filter::ScalarList(slf) => scalar_list_filter(slf, invert, invert_undefined_exclusion, prefix)?,
        Filter::OneRelationIsNull(filter) => one_is_null(filter, invert, prefix),
        Filter::Relation(rfilter) => relation_filter(rfilter.invert(invert), prefix)?,
        Filter::Aggregation(filter) => aggregation_filter(filter, invert, invert_undefined_exclusion)?,
        Filter::Composite(filter) => composite_filter(filter, invert, invert_undefined_exclusion, prefix)?,
        Filter::BoolFilter(_) => unimplemented!("MongoDB boolean filter."),
    };

    Ok(filter_pair)
}

fn fold_compounds(filter: Filter) -> Filter {
    match filter {
        Filter::Scalar(ScalarFilter {
            projection: ScalarProjection::Compound(fields),
            condition: ScalarCondition::In(value_tuples),
            mode: _,
        }) if fields.len() > 1 => {
            let mut filters = vec![];

            for tuple in value_tuples {
                let values = tuple.into_list().expect("Compounds must have associated value lists.");

                let equality_filters: Vec<_> = values
                    .into_iter()
                    .zip(fields.iter())
                    .map(|(value, field)| field.equals(value))
                    .collect();

                filters.push(Filter::And(equality_filters));
            }

            Filter::Or(filters)
        }
        _ => filter,
    }
}

fn coerce_empty(
    truthy: bool,
    operation: &str,
    filters: Vec<Filter>,
    invert: bool,
    invert_undefined_exclusion: bool,
    prefix: FilterPrefix,
) -> crate::Result<MongoFilter> {
    if filters.is_empty() {
        // We need to create a truthy or falsey expression for empty filter queries, e.g. AND / OR / NOT.
        // We abuse the fact that we can create an always failing or succeeding condition with logical `and` and `or` operators,
        // for example "a field exists or doesn't exist" is always true, "a field exists and doesn't exist" is always false.
        let stub_condition = render_stub_condition(truthy);

        Ok(MongoFilter::Scalar(stub_condition))
    } else {
        fold_filters(operation, filters, invert, invert_undefined_exclusion, prefix)
    }
}

fn fold_filters(
    operation: &str,
    filters: Vec<Filter>,
    invert: bool,
    invert_undefined_exclusion: bool,
    prefix: FilterPrefix,
) -> crate::Result<MongoFilter> {
    let filters = filters
        .into_iter()
        .map(|f| Ok(convert_filter_internal(f, invert, invert_undefined_exclusion, prefix.clone())?.render()))
        .collect::<crate::Result<Vec<_>>>()?;

    let (filters, joins) = fold_nested(filters);
    let filter_doc = doc! { operation: filters };

    Ok(MongoFilter::relation(filter_doc, joins))
}

// Todo we should really only join each relation once.
fn fold_nested(nested: Vec<(Document, Vec<JoinStage>)>) -> (Vec<Document>, Vec<JoinStage>) {
    nested.into_iter().fold((vec![], vec![]), |mut acc, next| {
        acc.0.push(next.0);
        acc.1.extend(next.1);
        acc
    })
}

fn scalar_filter(
    filter: ScalarFilter,
    invert: bool,
    invert_undefined_exclusion: bool,
    // Whether the scalar filter comes from an `AggregationFilter::Count(_)`
    is_count: bool,
    prefix: FilterPrefix,
) -> crate::Result<MongoFilter> {
    let field = match filter.projection {
        connector_interface::ScalarProjection::Single(sf) => sf,
        connector_interface::ScalarProjection::Compound(mut c) if c.len() == 1 => c.pop().unwrap(),
        connector_interface::ScalarProjection::Compound(_) => {
            unreachable!(
                "Multi-field compound filter case hit when it should have been folded into normal filters previously."
            )
        }
    };

    let filter = match filter.mode {
        QueryMode::Default => default_scalar_filter(
            &field,
            prefix,
            filter.condition.invert(invert),
            invert_undefined_exclusion,
            is_count,
        )?,
        QueryMode::Insensitive => insensitive_scalar_filter(&field, prefix, filter.condition.invert(invert))?,
    };

    Ok(MongoFilter::Scalar(filter))
}

// Note contains / startsWith / endsWith are only applicable to String types in the schema.
fn default_scalar_filter(
    field: &ScalarFieldRef,
    prefix: FilterPrefix,
    condition: ScalarCondition,
    invert_undefined_exclusion: bool,
    // Whether the scalar filter comes from an `AggregationFilter::Count(_)`
    is_count: bool,
) -> crate::Result<Document> {
    let field_name = prefix.render_with(field.db_name().to_owned());
    let is_set_cond = matches!(&condition, ScalarCondition::IsSet(_));

    let filter_doc = match condition {
        ScalarCondition::Equals(val) => {
            doc! { "$eq": [&field_name, into_bson_coerce_count(field, val, is_count)?] }
        }
        ScalarCondition::NotEquals(val) => {
            doc! { "$ne": [&field_name, into_bson_coerce_count(field, val, is_count)?] }
        }
        ScalarCondition::Contains(val) => regex_match(&field_name, field, ".*", val, ".*", false)?,
        ScalarCondition::NotContains(val) => {
            doc! { "$not": regex_match(&field_name, field, ".*", val, ".*", false)? }
        }
        ScalarCondition::StartsWith(val) => regex_match(&field_name, field, "^", val, "", false)?,
        ScalarCondition::NotStartsWith(val) => {
            doc! { "$not": regex_match(&field_name, field, "^", val, "", false)? }
        }
        ScalarCondition::EndsWith(val) => regex_match(&field_name, field, "", val, "$", false)?,
        ScalarCondition::NotEndsWith(val) => {
            doc! { "$not": regex_match(&field_name, field, "", val, "$", false)? }
        }
        ScalarCondition::LessThan(val) => {
            doc! { "$lt": [&field_name, into_bson_coerce_count(field, val, is_count)?] }
        }
        ScalarCondition::LessThanOrEquals(val) => {
            doc! { "$lte": [&field_name, into_bson_coerce_count(field, val, is_count)?] }
        }
        ScalarCondition::GreaterThan(val) => {
            doc! { "$gt": [&field_name, into_bson_coerce_count(field, val, is_count)?] }
        }
        ScalarCondition::GreaterThanOrEquals(val) => {
            doc! { "$gte": [&field_name, into_bson_coerce_count(field, val, is_count)?] }
        }
        // Todo: The nested list unpack looks like a bug somewhere.
        //       Likely join code mistakenly repacks a list into a list of PrismaValue somewhere in the core.
        ScalarCondition::In(vals) => match vals.split_first() {
            // List is list of lists, we need to flatten.
            Some((PrismaValue::List(_), _)) => {
                let mut bson_values = Vec::with_capacity(vals.len());

                for pv in vals {
                    if let PrismaValue::List(inner) = pv {
                        bson_values.extend(
                            inner
                                .into_iter()
                                .map(|val| into_bson_coerce_count(field, val, is_count))
                                .collect::<crate::Result<Vec<_>>>()?,
                        )
                    }
                }

                doc! { "$in": [&field_name, bson_values] }
            }
            _ => {
                doc! { "$in": [&field_name, into_bson_coerce_count(field, PrismaValue::List(vals), is_count)?] }
            }
        },
        ScalarCondition::NotIn(vals) => {
            let bson_values = vals
                .into_iter()
                .map(|val| into_bson_coerce_count(field, val, is_count))
                .collect::<crate::Result<Vec<_>>>()?;

            doc! { "$not": { "$in": [&field_name, bson_values] } }
        }
        ScalarCondition::JsonCompare(jc) => match *jc.condition {
            ScalarCondition::Equals(value) => {
                let bson = (field, value).into_bson()?;

                doc! { "$eq": [&field_name, bson] }
            }
            ScalarCondition::NotEquals(value) => {
                let bson = (field, value).into_bson()?;

                doc! { "$ne": [&field_name, bson] }
            }
            _ => unimplemented!("Only equality JSON filtering is supported on MongoDB."),
        },
        ScalarCondition::IsSet(is_set) => render_is_set(&field_name, is_set),
        ScalarCondition::Search(_, _) => unimplemented!("Full-text search is not supported yet on MongoDB"),
        ScalarCondition::NotSearch(_, _) => unimplemented!("Full-text search is not supported yet on MongoDB"),
    };

    let cond = if !is_set_cond {
        exclude_undefineds(&field_name, invert_undefined_exclusion, filter_doc)
    } else {
        filter_doc
    };

    Ok(cond)
}

/// Insensitive filters are only reachable with TypeIdentifier::String (or UUID, which is string as well for us).
fn insensitive_scalar_filter(
    field: &ScalarFieldRef,
    prefix: FilterPrefix,
    condition: ScalarCondition,
) -> crate::Result<Document> {
    let field_name = prefix.render_with(field.db_name().to_owned());

    match condition {
        ScalarCondition::Equals(val) => regex_match(&field_name, field, "^", val, "$", true),
        ScalarCondition::NotEquals(val) => Ok(doc! { "$not": regex_match(&field_name, field, "^", val, "$", true)? }),

        ScalarCondition::Contains(val) => regex_match(&field_name, field, ".*", val, ".*", true),
        ScalarCondition::NotContains(val) => {
            Ok(doc! { "$not": regex_match(&field_name, field, ".*", val, ".*", true)?})
        }
        ScalarCondition::StartsWith(val) => regex_match(&field_name, field, "^", val, "", true),
        ScalarCondition::NotStartsWith(val) => {
            Ok(doc! { "$not": regex_match(&field_name, field, "^", val, "", true)? })
        }
        ScalarCondition::EndsWith(val) => regex_match(&field_name, field, "", val, "$", true),
        ScalarCondition::NotEndsWith(val) => Ok(doc! { "$not": regex_match(&field_name, field, "", val, "$", true)? }),
        ScalarCondition::LessThan(val) => Ok(doc! { "$lt": [&field_name, (field, val).into_bson()?] }),
        ScalarCondition::LessThanOrEquals(val) => Ok(doc! { "$lte": [&field_name, (field, val).into_bson()?] }),
        ScalarCondition::GreaterThan(val) => Ok(doc! { "$gt": [&field_name, (field, val).into_bson()?] }),
        ScalarCondition::GreaterThanOrEquals(val) => Ok(doc! { "$gte": [&field_name, (field, val).into_bson()?] }),
        // Todo: The nested list unpack looks like a bug somewhere.
        // Likely join code mistakenly repacks a list into a list of PrismaValue somewhere in the core.
        ScalarCondition::In(vals) => match vals.split_first() {
            // List is list of lists, we need to flatten.
            Some((PrismaValue::List(_), _)) => {
                let mut matches = Vec::with_capacity(vals.len());

                for pv in vals {
                    if let PrismaValue::List(inner) = pv {
                        for val in inner {
                            matches.push(regex_match(&field_name, field, "^", val, "$", true)?)
                        }
                    }
                }

                Ok(doc! { "$or": matches })
            }

            _ => {
                let matches = vals
                    .into_iter()
                    .map(|val| regex_match(&field_name, field, "^", val, "$", true))
                    .collect::<crate::Result<Vec<_>>>()?;

                Ok(doc! { "$or": matches })
            }
        },
        ScalarCondition::NotIn(vals) => {
            let matches = vals
                .into_iter()
                .map(|val| regex_match(&field_name, field, "^", val, "$", true).map(|doc| doc! { "$not": doc }))
                .collect::<crate::Result<Vec<_>>>()?;

            Ok(doc! { "$and": matches })
        }
        ScalarCondition::IsSet(is_set) => Ok(render_is_set(&field_name, is_set)),
        ScalarCondition::JsonCompare(_) => Err(MongoError::Unsupported(
            "JSON filtering is not yet supported on MongoDB".to_string(),
        )),
        ScalarCondition::Search(_, _) | ScalarCondition::NotSearch(_, _) => Err(MongoError::Unsupported(
            "Full-text search is not supported yet on MongoDB".to_string(),
        )),
    }
}

/// Filters available on list fields.
fn scalar_list_filter(
    filter: ScalarListFilter,
    invert: bool,
    invert_undefined_exclusion: bool,
    prefix: FilterPrefix,
) -> crate::Result<MongoFilter> {
    let field = filter.field;
    let field_name = prefix.render_with(field.db_name().into());

    // Of course Mongo needs special filters for the inverted case, everything else would be too easy.
    let filter_doc = if invert {
        match filter.condition {
            // "Contains element" -> "Does not contain element"
            connector_interface::ScalarListCondition::Contains(val) => {
                doc! { "$not": { "$in": [(&field, val).into_bson()?, coerce_as_array(&field_name)] } }
            }

            // "Contains all elements" -> "Does not contain any of the elements"
            connector_interface::ScalarListCondition::ContainsEvery(vals) => {
                let ins = vals
                    .into_iter()
                    .map(|val| {
                        (&field, val)
                            .into_bson()
                            .map(|bson_val| doc! { "$not": { "$in": [bson_val, coerce_as_array(&field_name)] } })
                    })
                    .collect::<crate::Result<Vec<_>>>()?;

                doc! {
                    "$and": ins
                }
            }

            // "Contains some of the elements" -> "Does not contain some of the elements"
            connector_interface::ScalarListCondition::ContainsSome(vals) => {
                let ins = vals
                    .into_iter()
                    .map(|val| {
                        (&field, val)
                            .into_bson()
                            .map(|bson_val| doc! { "$not": { "$in": [bson_val, coerce_as_array(&field_name)] } })
                    })
                    .collect::<crate::Result<Vec<_>>>()?;

                doc! {
                    "$or": ins
                }
            }

            // Empty -> not empty and vice versa
            connector_interface::ScalarListCondition::IsEmpty(should_be_empty) => {
                if should_be_empty && !invert {
                    doc! { "$eq": [render_size(&field_name, true), 0] }
                } else {
                    doc! { "$gt": [render_size(&field_name, true), 0] }
                }
            }
        }
    } else {
        match filter.condition {
            connector_interface::ScalarListCondition::Contains(val) => {
                doc! { "$in": [(&field, val).into_bson()?, coerce_as_array(&field_name)] }
            }

            connector_interface::ScalarListCondition::ContainsEvery(vals) if vals.is_empty() => {
                // Empty hasEvery: Return all records.
                render_stub_condition(true)
            }

            connector_interface::ScalarListCondition::ContainsEvery(vals) => {
                let ins = vals
                    .into_iter()
                    .map(|val| {
                        (&field, val)
                            .into_bson()
                            .map(|bson_val| doc! { "$in": [bson_val, coerce_as_array(&field_name)] })
                    })
                    .collect::<crate::Result<Vec<_>>>()?;

                doc! { "$and": ins }
            }

            connector_interface::ScalarListCondition::ContainsSome(vals) if vals.is_empty() => {
                // Empty hasSome: Return no records.
                render_stub_condition(false)
            }

            connector_interface::ScalarListCondition::ContainsSome(vals) => {
                let ins = vals
                    .into_iter()
                    .map(|val| {
                        (&field, val)
                            .into_bson()
                            .map(|bson_val| doc! { "$in": [bson_val, coerce_as_array(&field_name)] })
                    })
                    .collect::<crate::Result<Vec<_>>>()?;

                doc! { "$or": ins }
            }

            connector_interface::ScalarListCondition::IsEmpty(should_be_empty) => {
                if should_be_empty {
                    doc! { "$eq": [render_size(&field_name, true), 0] }
                } else {
                    doc! { "$gt": [render_size(&field_name, true), 0] }
                }
            }
        }
    };

    let filter_doc = exclude_undefineds(&field_name, invert_undefined_exclusion, filter_doc);

    Ok(MongoFilter::Scalar(filter_doc))
}

// Can be optimized by checking inlined fields on the left side instead of always joining.
fn one_is_null(filter: OneRelationIsNullFilter, invert: bool, prefix: FilterPrefix) -> MongoFilter {
    let rf = filter.field;
    let field_name = prefix.render_with(rf.relation().name.to_owned());
    let join_stage = JoinStage::new(rf);

    let filter_doc = if invert {
        doc! { "$gt": [render_size(&field_name, false), 0] }
    } else {
        doc! { "$eq": [render_size(&field_name, false), 0] }
    };

    MongoFilter::relation(filter_doc, vec![join_stage])
}

/// Builds a Mongo relation filter depth-first.
fn relation_filter(filter: RelationFilter, prefix: FilterPrefix) -> crate::Result<MongoFilter> {
    let from_field = filter.field;
    let nested_filter = *filter.nested_filter;
    let is_to_one = !from_field.is_list();
    let field_name = prefix.render_with(from_field.relation().name.to_owned());
    // Tmp condition check while mongo is getting fully tested.
    let is_empty_filter = matches!(nested_filter, Filter::Empty);

    let mut join_stage = JoinStage::new(from_field);

    let filter_doc = match filter.condition {
        connector_interface::RelationCondition::EveryRelatedRecord => {
            let (every, nested_joins) = render_every(&field_name, nested_filter, false, false)?;

            join_stage.extend_nested(nested_joins);

            every
        }
        connector_interface::RelationCondition::AtLeastOneRelatedRecord => {
            let (some, nested_joins) = render_some(&field_name, nested_filter, false, false)?;

            join_stage.extend_nested(nested_joins);

            some
        }
        connector_interface::RelationCondition::NoRelatedRecord if is_to_one => {
            if is_empty_filter {
                // Doesn't need coercing the array since joins always return arrays
                doc! { "$eq": [render_size(&field_name, false), 0] }
            } else {
                let (none, nested_joins) = render_none(&field_name, nested_filter, true, false)?;

                join_stage.extend_nested(nested_joins);

                // If the relation is a to-one, ensure the array is of size 1
                // This filters out undefined to-one relations
                doc! {
                    "$and": [
                        none,
                        // Additionally, we ensure that the array has a single element.
                        // It doesn't need to be coerced to an empty array since the join guarantees it will exist
                        { "$eq": [render_size(&field_name, false), 1] }
                    ]
                }
            }
        }
        connector_interface::RelationCondition::NoRelatedRecord => {
            if is_empty_filter {
                // Doesn't need coercing the array since joins always return arrays
                doc! { "$eq": [render_size(&field_name, false), 0] }
            } else {
                let (none, nested_joins) = render_none(&field_name, nested_filter, true, false)?;

                join_stage.extend_nested(nested_joins);

                none
            }
        }
        connector_interface::RelationCondition::ToOneRelatedRecord => {
            // To-ones are coerced to single-element arrays via the join.
            // We render an "every" expression on that array to ensure that the predicate is matched.
            let (every, nested_joins) = render_every(&field_name, nested_filter, false, false)?;

            join_stage.extend_nested(nested_joins);

            doc! {
                "$and": [
                    every,
                    // Additionally, we ensure that the array has a single element.
                    // It doesn't need to be coerced to an empty array since the join guarantees it will exist
                    { "$eq": [render_size(&field_name, false), 1] }
                ]
            }
        }
    };

    Ok(MongoFilter::relation(filter_doc, vec![join_stage]))
}

fn aggregation_filter(
    filter: AggregationFilter,
    invert: bool,
    invert_undefined_exclusion: bool,
) -> crate::Result<MongoFilter> {
    match filter {
        AggregationFilter::Count(filter) => aggregate_conditions("count", *filter, invert, invert_undefined_exclusion),
        AggregationFilter::Average(filter) => aggregate_conditions("avg", *filter, invert, invert_undefined_exclusion),
        AggregationFilter::Sum(filter) => aggregate_conditions("sum", *filter, invert, invert_undefined_exclusion),
        AggregationFilter::Min(filter) => aggregate_conditions("min", *filter, invert, invert_undefined_exclusion),
        AggregationFilter::Max(filter) => aggregate_conditions("max", *filter, invert, invert_undefined_exclusion),
    }
}

fn aggregate_conditions(
    op: &str,
    filter: Filter,
    invert: bool,
    invert_undefined_exclusion: bool,
) -> crate::Result<MongoFilter> {
    let sf = match filter {
        Filter::Scalar(sf) => sf,
        _ => unimplemented!(),
    };

    let field = match &sf.projection {
        ScalarProjection::Single(field) => field,
        _ => unreachable!(),
    };

    let mut prefix = FilterPrefix::from(format!("{}_{}", op, field.db_name()));
    // An aggregation filter can only refer to its aggregated field, which is already the "target".
    // Therefore, we make sure the additional target in `scalar_filter` won't be rendered.
    prefix.ignore_target(true);

    let (filter, _) = scalar_filter(sf, invert, invert_undefined_exclusion, op == "count", prefix)?.render();

    Ok(MongoFilter::Scalar(filter))
}

fn composite_filter(
    filter: CompositeFilter,
    invert: bool,
    invert_undefined_exclusion: bool,
    prefix: FilterPrefix,
) -> crate::Result<MongoFilter> {
    let field = filter.field;
    let composite_name = field.db_name();
    let field_name = prefix.clone().render_with(composite_name.to_string());
    let is_set_cond = matches!(*filter.condition, CompositeCondition::IsSet(_));

    let filter_doc = match *filter.condition {
        CompositeCondition::Every(filter) => {
            // let is_empty = matches!(filter, Filter::Empty);
            let (every, _) = render_every(&field_name, filter, invert_undefined_exclusion, true)?;

            every
        }

        CompositeCondition::Some(filter) => {
            let (some, _) = render_some(&field_name, filter, invert_undefined_exclusion, true)?;

            some
        }

        CompositeCondition::None(filter) => {
            let (none, _) = render_none(&field_name, filter, !invert_undefined_exclusion, true)?;

            none
        }

        CompositeCondition::Equals(value) => {
            doc! { "$eq": [&field_name, (&field, value).into_bson()?] }
        }

        CompositeCondition::Empty(should_be_empty) => {
            let empty_doc = if should_be_empty {
                doc! { "$eq": [render_size(&field_name, true), 0] }
            } else {
                doc! { "$gt": [render_size(&field_name, true), 0] }
            };

            if invert {
                doc! {
                    "$or": [
                        empty_doc,
                        doc! { "$eq": [coerce_as_null(&field_name), null] }
                    ]
                }
            } else {
                doc! {
                    "$and": [
                        empty_doc,
                        doc! { "$ne": [coerce_as_null(&field_name), null] }
                    ]
                }
            }
        }

        CompositeCondition::IsSet(is_set) => render_is_set(&field_name, is_set),
        CompositeCondition::Is(filter) => {
            let (nested_filter, _) = convert_filter_internal(
                filter,
                invert,
                invert_undefined_exclusion,
                prefix.append_cloned(field.db_name()),
            )?
            .render();

            return Ok(MongoFilter::Composite(nested_filter));
        }

        CompositeCondition::IsNot(filter) => {
            let (nested_filter, _) = convert_filter_internal(
                filter,
                !invert,
                invert_undefined_exclusion,
                prefix.append_cloned(field.db_name()),
            )?
            .render();

            return Ok(MongoFilter::Composite(nested_filter));
        }
    };

    let filter_doc = if invert {
        doc! { "$not": filter_doc }
    } else {
        filter_doc
    };

    let filter_doc = if !is_set_cond {
        exclude_undefineds(&field_name, invert_undefined_exclusion, filter_doc)
    } else {
        filter_doc
    };

    Ok(MongoFilter::Composite(filter_doc))
}

/// Renders a `$regexMatch` expression.
fn regex_match(
    field_name: &str,
    field: &ScalarFieldRef,
    prefix: &str,
    val: PrismaValue,
    suffix: &str,
    insensitive: bool,
) -> crate::Result<Document> {
    let options = if insensitive { "i" } else { "" }.to_owned();
    let pattern = format!(
        "{}{}{}",
        prefix,
        (field, val)
            .into_bson()?
            .as_str()
            .expect("Only reachable with String types."),
        suffix
    );

    Ok(doc! {
        "$regexMatch": {
            "input": field_name,
            "regex": pattern,
            "options": options
        }
    })
}

/// Renders a `$size` expression to compute the length of an array.
/// If `coerce_array` is true, the array will be coerced to an empty array in case it's `null` or `undefined`.
fn render_size(field_name: &str, coerce_array: bool) -> Document {
    if coerce_array {
        doc! { "$size": coerce_as_array(field_name) }
    } else {
        doc! { "$size": field_name }
    }
}

/// Coerces a field to an empty array if it's `null` or `undefined`.
/// Renders an `$ifNull` expression.
fn coerce_as_array(field_name: &str) -> Document {
    doc! { "$ifNull": [field_name, []] }
}

/// Coerces a field to `null` if it's `null` or `undefined`.
/// Used to convert `undefined` fields to `null`.
/// Renders an `$ifNull` expression.
fn coerce_as_null(field_name: &str) -> Document {
    doc! { "$ifNull": [field_name, null] }
}

/// Renders an expression that computes whether _some_ of the elements of an array matches the `Filter`.
/// If `coerce_array` is true, the array will be coerced to an empty array in case it's `null` or `undefined`.
fn render_some(
    field_name: &str,
    filter: Filter,
    invert_undefined_exclusion: bool,
    coerce_array: bool,
) -> crate::Result<(Document, Vec<JoinStage>)> {
    let input = if coerce_array {
        Bson::from(coerce_as_array(field_name))
    } else {
        Bson::from(field_name)
    };

    // Nested filters needs to be prefixed with `$$elem` so that they refer to the "elem" alias defined in the $filter operator below.
    let prefix = FilterPrefix::from("$elem");
    let (nested_filter, nested_joins) =
        convert_filter_internal(filter, false, invert_undefined_exclusion, prefix)?.render();

    let doc = doc! {
      "$gt": [
        {
          "$size": {
            "$filter": {
              "input": input,
              "as": "elem",
              "cond": nested_filter
            }
          }
        },
        0
      ]
    };

    Ok((doc, nested_joins))
}

/// Renders an expression that computes whether _all_ of the elements of an array matches the `Filter`.
/// If `coerce_array` is true, the array will be coerced to an empty array in case it's `null` or `undefined`.
fn render_every(
    field_name: &str,
    filter: Filter,
    invert_undefined_exclusion: bool,
    coerce_array: bool,
) -> crate::Result<(Document, Vec<JoinStage>)> {
    let input = if coerce_array {
        Bson::from(coerce_as_array(field_name))
    } else {
        Bson::from(field_name)
    };

    // Nested filters needs to be prefixed with `$$elem` so that they refer to the "elem" alias defined in the $filter operator below.
    let prefix = FilterPrefix::from("$elem");
    let (nested_filter, nested_joins) =
        convert_filter_internal(filter, false, invert_undefined_exclusion, prefix)?.render();

    let doc = doc! {
      "$eq": [
        {
          "$size": {
            "$filter": {
              "input": input,
              "as": "elem",
              "cond": nested_filter,
            }
          }
        },
        render_size(field_name, true)
      ]
    };

    Ok((doc, nested_joins))
}

/// Renders an expression that computes whether _none_ of the elements of an array matches the `Filter`.
/// If `coerce_array` is true, the array will be coerced to an empty array in case it's `null` or `undefined`.
fn render_none(
    field_name: &str,
    filter: Filter,
    invert_undefined_exclusion: bool,
    coerce_array: bool,
) -> crate::Result<(Document, Vec<JoinStage>)> {
    let input = if coerce_array {
        Bson::from(coerce_as_array(field_name))
    } else {
        Bson::from(field_name)
    };

    // Nested filters needs to be prefixed with `$$elem` so that they refer to the "elem" alias defined in the $filter operator below.
    let prefix = FilterPrefix::from("$elem");
    let (nested_filter, nested_joins) =
        convert_filter_internal(filter, false, invert_undefined_exclusion, prefix)?.render();

    let doc = doc! {
      "$eq": [
        {
          "$size": {
            "$filter": {
              "input": input,
              "as": "elem",
              "cond": nested_filter
            }
          }
        },
        0
      ]
    };

    Ok((doc, nested_joins))
}

/// Renders a stub condition that's either true or false
fn render_stub_condition(truthy: bool) -> Document {
    doc! { "$and": truthy }
}

fn render_is_set(field_name: &str, is_set: bool) -> Document {
    if is_set {
        // To check whether a field is undefined, we need to coerce it to `null` first.
        // This is why we _also_ need to check whether the field is equal to null
        doc! {
            "$or": [
                { "$ne": [coerce_as_null(field_name), null] },
                { "$eq": [field_name, null] }
              ]
        }
    } else {
        doc! {
            "$and": [
                { "$eq": [coerce_as_null(&field_name), null] },
                { "$ne": [&field_name, null] }
              ]
        }
    }
}

fn exclude_undefineds(field_name: &str, invert: bool, filter: Document) -> Document {
    let is_set_filter = render_is_set(field_name, !invert);

    if invert {
        doc! { "$or": [filter, is_set_filter] }
    } else {
        doc! { "$and": [filter, is_set_filter] }
    }
}

/// Convert a PrismaValue into Bson, with a special case for `_count` aggregation filter.
///
/// When converting the value of a `_count` aggregation filter for a field that's _not_ numerical,
/// we force the `TypeIdentifier` to be `Int` to prevent panics.
fn into_bson_coerce_count(sf: &ScalarFieldRef, value: PrismaValue, is_count_aggregation: bool) -> crate::Result<Bson> {
    if is_count_aggregation && !sf.is_numeric() {
        (&TypeIdentifier::Int, value).into_bson()
    } else {
        (sf, value).into_bson()
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct FilterPrefix {
    parts: Vec<String>,
    /// Whether the `target` should be rendered by the `render_with` method
    ignore_target: bool,
}

impl FilterPrefix {
    pub fn append_cloned<T>(&self, elem: T) -> Self
    where
        T: Into<String>,
    {
        let mut new = self.clone();

        new.parts.push(elem.into());
        new
    }

    pub fn render(self) -> String {
        self.parts.join(".")
    }

    pub fn render_with(self, target: String) -> String {
        if self.ignore_target {
            return format!("${}", self.render());
        }

        if self.parts.is_empty() {
            format!("${}", target)
        } else {
            format!("${}.{}", self.render(), target)
        }
    }

    /// Sets whether the target should be rendered by the `render_with` method
    pub fn ignore_target(&mut self, ignore_target: bool) {
        self.ignore_target = ignore_target;
    }
}

impl From<&CompositeFieldRef> for FilterPrefix {
    fn from(cf: &CompositeFieldRef) -> Self {
        Self {
            parts: vec![cf.db_name().to_owned()],
            ignore_target: false,
        }
    }
}

impl From<String> for FilterPrefix {
    fn from(alias: String) -> Self {
        Self {
            parts: vec![alias],
            ignore_target: false,
        }
    }
}

impl From<&str> for FilterPrefix {
    fn from(alias: &str) -> Self {
        Self {
            parts: vec![alias.to_owned()],
            ignore_target: false,
        }
    }
}
