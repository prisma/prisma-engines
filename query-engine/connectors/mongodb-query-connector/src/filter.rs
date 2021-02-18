use crate::IntoBson;
use connector_interface::{Filter, QueryMode, ScalarCondition, ScalarFilter, ScalarListFilter};
use mongodb::bson::{doc, Bson, Document, Regex};
use prisma_models::{PrismaValue, ScalarFieldRef};
use std::unimplemented;

/// Builds a MongoDB query document from a Prisma filter.
/// Returns the query document and a number of join documents
/// if required, e.g. for relation filters.
pub fn convert_filter(filter: Filter) -> crate::Result<(Document, Vec<Document>)> {
    let filter_pair = match filter {
        Filter::And(filters) => filter_list("$and", filters)?,
        Filter::Or(filters) => filter_list("$or", filters)?,
        Filter::Not(filters) => filter_list("$not", filters)?,
        Filter::Scalar(sf) => (scalar_filter(sf)?, vec![]),
        Filter::Empty => (Document::new(), vec![]),
        Filter::ScalarList(slf) => (scalar_list_filter(slf)?, vec![]),
        // Filter::OneRelationIsNull(_) => {}
        // Filter::Relation(_) => {}
        // Filter::BoolFilter(b) => {} // Potentially not doable.
        // Filter::Aggregation(_) => {}
        _ => todo!("Incomplete filter implementation."),
    };

    Ok(filter_pair)
}

fn filter_list(operation: &str, filters: Vec<Filter>) -> crate::Result<(Document, Vec<Document>)> {
    let filters = filters
        .into_iter()
        .map(|f| convert_filter(f))
        .collect::<crate::Result<Vec<_>>>()?;

    let (filters, joins) = fold_nested(filters);

    Ok((doc! { operation: filters }, joins))
}

// Todo we should really only join each relation once.
fn fold_nested(nested: Vec<(Document, Vec<Document>)>) -> (Vec<Document>, Vec<Document>) {
    nested.into_iter().fold((vec![], vec![]), |mut acc, next| {
        acc.0.push(next.0);
        acc.1.extend(next.1);
        acc
    })
}

fn scalar_filter(filter: ScalarFilter) -> crate::Result<Document> {
    // Todo: Find out what Compound cases are really. (Guess: Relation fields with multi-field FK?)
    let field = match filter.projection {
        connector_interface::ScalarProjection::Single(sf) => sf,
        connector_interface::ScalarProjection::Compound(mut c) if c.len() == 1 => c.pop().unwrap(),
        connector_interface::ScalarProjection::Compound(_) => unimplemented!("Compound filter case."),
    };

    let filter = match filter.mode {
        QueryMode::Default => default_scalar_filter(&field, filter.condition)?,
        QueryMode::Insensitive => insensitive_scalar_filter(&field, filter.condition)?,
    };

    Ok(doc! { field.db_name(): filter })
}

// Note contains / startsWith / endsWith are only applicable to String types in the schema.
fn default_scalar_filter(field: &ScalarFieldRef, condition: ScalarCondition) -> crate::Result<Document> {
    Ok(match condition {
        ScalarCondition::Equals(val) => doc! { "$eq": (field, val).into_bson()? },
        ScalarCondition::NotEquals(val) => doc! { "$ne": (field, val).into_bson()? },
        ScalarCondition::Contains(val) => doc! { "$regex": to_regex(field, ".*", val, ".*", false)? },
        ScalarCondition::NotContains(val) => doc! { "$not": { "$regex": to_regex(field, ".*", val, ".*", false)? }},
        ScalarCondition::StartsWith(val) => doc! { "$regex": to_regex(field, "^", val, "", false)? },
        ScalarCondition::NotStartsWith(val) => doc! { "$not": { "$regex": to_regex(field, "^", val, "", false)? }},
        ScalarCondition::EndsWith(val) => doc! { "$regex": to_regex(field, "", val, "$", false)? },
        ScalarCondition::NotEndsWith(val) => doc! { "$not": { "$regex": to_regex(field, "", val, "$", false)? }},
        ScalarCondition::LessThan(val) => doc! { "$lt": (field, val).into_bson()? },
        ScalarCondition::LessThanOrEquals(val) => doc! { "$lte": (field, val).into_bson()? },
        ScalarCondition::GreaterThan(val) => doc! { "$gt": (field, val).into_bson()? },
        ScalarCondition::GreaterThanOrEquals(val) => doc! { "$gte": (field, val).into_bson()? },
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
                                .map(|val| (field, val).into_bson())
                                .collect::<crate::Result<Vec<_>>>()?,
                        )
                    }
                }

                doc! { "$in": bson_values }
            }
            _ => doc! { "$in": (field, PrismaValue::List(vals)).into_bson()? },
        },
        ScalarCondition::NotIn(vals) => {
            doc! { "$nin": vals.into_iter().map(|val| (field, val).into_bson()).collect::<crate::Result<Vec<_>>>()? }
        }
    })
}

/// Insensitive filters are only reachable with TypeIdentifier::String (or UUID, which is string as well for us).
fn insensitive_scalar_filter(field: &ScalarFieldRef, condition: ScalarCondition) -> crate::Result<Document> {
    Ok(match condition {
        ScalarCondition::Equals(val) => doc! { "$regex": to_regex(field, "^", val, "$", true)? },
        ScalarCondition::NotEquals(val) => {
            doc! { "$not": { "$regex": to_regex(field, "^", val, "$", true)? }}
        }

        ScalarCondition::Contains(val) => doc! { "$regex": to_regex(field, ".*", val, ".*", true)? },
        ScalarCondition::NotContains(val) => doc! { "$not": { "$regex": to_regex(field, ".*", val, ".*", true)? }},
        ScalarCondition::StartsWith(val) => doc! { "$regex": to_regex(field, "^", val, "", true)?  },
        ScalarCondition::NotStartsWith(val) => doc! { "$not": { "$regex": to_regex(field, "^", val, "", true)? }},
        ScalarCondition::EndsWith(val) => doc! { "$regex": to_regex(field, "", val, "$", true)? },
        ScalarCondition::NotEndsWith(val) => doc! { "$not": { "$regex": to_regex(field, "", val, "$", true)? }},
        ScalarCondition::LessThan(val) => doc! { "$lt": (field, val).into_bson()? },
        ScalarCondition::LessThanOrEquals(val) => doc! { "$lte": (field, val).into_bson()? },
        ScalarCondition::GreaterThan(val) => doc! { "$gt": (field, val).into_bson()? },
        ScalarCondition::GreaterThanOrEquals(val) => doc! { "$gte": (field, val).into_bson()? },
        // Todo: The nested list unpack looks like a bug somewhere.
        //       Likely join code mistakenly repacks a list into a list of PrismaValue somewhere in the core.
        ScalarCondition::In(vals) => match vals.split_first() {
            // List is list of lists, we need to flatten.
            Some((PrismaValue::List(_), _)) => {
                let mut bson_values = Vec::with_capacity(vals.len());

                for pv in vals {
                    if let PrismaValue::List(inner) = pv {
                        bson_values.extend(to_regex_list(field, "^", inner, "$", true)?)
                    }
                }

                doc! { "$in": bson_values }
            }

            _ => doc! { "$in": to_regex_list(field, "^", vals, "$", true)? },
        },
        ScalarCondition::NotIn(vals) => {
            doc! { "$nin": to_regex_list(field, "^", vals, "$", true)? }
        }
    })
}

fn scalar_list_filter(filter: ScalarListFilter) -> crate::Result<Document> {
    let field = filter.field;

    let filter_doc = match filter.condition {
        connector_interface::ScalarListCondition::Contains(val) => {
            doc! { field.db_name(): (&field, val).into_bson()? }
        }

        connector_interface::ScalarListCondition::ContainsEvery(vals) => {
            doc! { field.db_name(): { "$all": (&field, PrismaValue::List(vals)).into_bson()? }}
        }

        connector_interface::ScalarListCondition::ContainsSome(vals) => {
            doc! { "$or": vals.into_iter().map(|val| Ok(doc! { field.db_name(): (&field, val).into_bson()? }) ).collect::<crate::Result<Vec<_>>>()?}
        }

        connector_interface::ScalarListCondition::IsEmpty(empty) => {
            if empty {
                doc! { field.db_name(): { "$size": 0 }}
            } else {
                doc! { field.db_name(): { "$not": { "$size": 0 }}}
            }
        }
    };

    Ok(filter_doc)
}

fn to_regex_list(
    field: &ScalarFieldRef,
    prefix: &str,
    vals: Vec<PrismaValue>,
    suffix: &str,
    insensitive: bool,
) -> crate::Result<Vec<Bson>> {
    vals.into_iter()
        .map(|val| to_regex(field, prefix, val, suffix, insensitive))
        .collect::<crate::Result<Vec<_>>>()
}

fn to_regex(
    field: &ScalarFieldRef,
    prefix: &str,
    val: PrismaValue,
    suffix: &str,
    insensitive: bool,
) -> crate::Result<Bson> {
    let options = if insensitive { "i" } else { "" }.to_owned();

    Ok(Bson::RegularExpression(Regex {
        pattern: format!(
            "{}{}{}",
            prefix,
            (field, val)
                .into_bson()?
                .as_str()
                .expect("Only reachable with String types."),
            suffix
        ),
        options,
    }))
}
