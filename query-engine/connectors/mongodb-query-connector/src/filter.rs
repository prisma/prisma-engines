use crate::{join::JoinStage, IntoBson};
use connector_interface::{
    Filter, OneRelationIsNullFilter, QueryMode, RelationCondition, RelationFilter, ScalarCondition, ScalarFilter,
    ScalarListFilter,
};
use mongodb::bson::{doc, Bson, Document, Regex};
use prisma_models::{PrismaValue, ScalarFieldRef};

#[derive(Debug)]
pub(crate) enum MongoFilter {
    Scalar(Document),
    Relation(MongoRelationFilter),
}

impl MongoFilter {
    pub(crate) fn render(self) -> (Document, Vec<JoinStage>) {
        match self {
            Self::Scalar(document) => (document, vec![]),
            Self::Relation(rf) => (rf.filter, rf.joins),
        }
    }

    pub(crate) fn relation(filter: Document, joins: Vec<JoinStage>) -> Self {
        Self::Relation(MongoRelationFilter { filter: filter, joins })
    }
}

#[derive(Debug)]
pub(crate) struct MongoRelationFilter {
    /// The filter that has to be applied to this layer of nesting (after all joins on this layer are done).
    pub filter: Document,

    /// All join trees required on this level to make the above filter work.
    pub joins: Vec<JoinStage>, // todo this is confusing, because in the "merged" state this will always be len = 1.
}

/// Builds a MongoDB query filter from a Prisma filter.
pub(crate) fn convert_filter(filter: Filter, invert: bool) -> crate::Result<MongoFilter> {
    let filter_pair = match filter {
        Filter::And(filters) if invert => compound_filter("$or", filters, invert)?,
        Filter::And(filters) => compound_filter("$and", filters, invert)?,

        Filter::Or(filters) if invert => compound_filter("$and", filters, invert)?,
        Filter::Or(filters) => compound_filter("$or", filters, invert)?,

        // todo requires some more testing
        Filter::Not(filters) => compound_filter("$and", filters, !invert)?,

        Filter::Scalar(sf) => scalar_filter(sf, invert)?,
        Filter::Empty => MongoFilter::Scalar(doc! {}),
        Filter::ScalarList(slf) => scalar_list_filter(slf, invert)?,
        Filter::OneRelationIsNull(filter) => one_is_null(filter, invert)?,
        Filter::Relation(rfilter) => relation_filter(rfilter, invert)?,
        // Filter::BoolFilter(b) => {} // Potentially not doable.
        // Filter::Aggregation(_) => {}
        _ => todo!("Incomplete filter implementation."),
    };

    Ok(filter_pair)
}

fn compound_filter(operation: &str, filters: Vec<Filter>, invert: bool) -> crate::Result<MongoFilter> {
    let filters = filters
        .into_iter()
        .map(|f| Ok(convert_filter(f, invert)?.render()))
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

fn scalar_filter(filter: ScalarFilter, invert: bool) -> crate::Result<MongoFilter> {
    // Todo: Find out what Compound cases are really. (Guess: Relation fields with multi-field FK?)
    let field = match filter.projection {
        connector_interface::ScalarProjection::Single(sf) => sf,
        connector_interface::ScalarProjection::Compound(mut c) if c.len() == 1 => c.pop().unwrap(),
        connector_interface::ScalarProjection::Compound(_) => unimplemented!("Compound filter case."),
    };

    let filter = match filter.mode {
        QueryMode::Default => default_scalar_filter(&field, filter.condition.invert(invert))?,
        QueryMode::Insensitive => insensitive_scalar_filter(&field, filter.condition.invert(invert))?,
    };

    Ok(MongoFilter::Scalar(doc! { field.db_name(): filter }))
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

/// Filters available on list fields.
fn scalar_list_filter(filter: ScalarListFilter, invert: bool) -> crate::Result<MongoFilter> {
    let field = filter.field;

    // Of course Mongo needs special filters for the inverted case, everything else would be too easy.
    let filter_doc = if invert {
        match filter.condition {
            // "Contains element" -> "Does not contain element"
            connector_interface::ScalarListCondition::Contains(val) => {
                doc! { field.db_name(): { "$elemMatch": { "$not": { "$eq": (&field, val).into_bson()? }}}}
            }

            // "Contains all elements" -> "Does not contain any of the elements"
            connector_interface::ScalarListCondition::ContainsEvery(vals) => {
                doc! { field.db_name(): { "$nin": (&field, PrismaValue::List(vals)).into_bson()? }}
            }

            // "Contains some of the elements" -> "Does not contain some of the elements"
            connector_interface::ScalarListCondition::ContainsSome(vals) => {
                doc! { field.db_name(): { "$elemMatch": { "$not": { "$in": (&field, PrismaValue::List(vals)).into_bson()? }}}}
            }

            // Empty -> not empty and vice versa
            connector_interface::ScalarListCondition::IsEmpty(check_for_empty) => {
                if check_for_empty && !invert {
                    doc! { field.db_name(): { "$size": 0 }}
                } else {
                    doc! { field.db_name(): { "$not": { "$size": 0 }}}
                }
            }
        }
    } else {
        match filter.condition {
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
        }
    };

    Ok(MongoFilter::Scalar(filter_doc))
}

// Can be optimized by checking inlined fields on the left side instead of always joining.
fn one_is_null(filter: OneRelationIsNullFilter, invert: bool) -> crate::Result<MongoFilter> {
    let rf = filter.field;
    let relation_name = &rf.relation().name;
    let join_stage = JoinStage::new(rf);

    let filter_doc = if invert {
        doc! { relation_name: { "$not": { "$size": 0 }}}
    } else {
        doc! { relation_name: { "$size": 0 }}
    };

    Ok(MongoFilter::relation(filter_doc, vec![join_stage]))
}

/// Builds a Mongo relation filter depth-first.
fn relation_filter(filter: RelationFilter, invert: bool) -> crate::Result<MongoFilter> {
    let from_field = filter.field;
    let relation_name = &from_field.relation().name;

    // `invert` xor `filter requires invert`
    let (nested_filter, nested_joins) =
        convert_filter(*filter.nested_filter, invert ^ requires_invert(&filter.condition))?.render();

    let mut join_stage = JoinStage::new(from_field);
    join_stage.add_all_nested(nested_joins);

    let filter_doc = match filter.condition {
        connector_interface::RelationCondition::EveryRelatedRecord => {
            doc! { "$all": [{ "$elemMatch": nested_filter }]}
        }
        connector_interface::RelationCondition::AtLeastOneRelatedRecord => {
            doc! { "$elemMatch": nested_filter }
        }
        connector_interface::RelationCondition::NoRelatedRecord => {
            doc! { "$all": [{ "$elemMatch": nested_filter }]}
        }
        connector_interface::RelationCondition::ToOneRelatedRecord => {
            doc! { "$all": [{ "$elemMatch": nested_filter }]}
        }
    };

    Ok(MongoFilter::relation(
        doc! { relation_name: filter_doc },
        vec![join_stage],
    ))
}

/// Checks if the given relation filter condition needs an inherent invert for MongoDB.
fn requires_invert(rf: &RelationCondition) -> bool {
    match rf {
        RelationCondition::NoRelatedRecord => true,
        _ => false,
    }
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
