use crate::{error::MongoError, join::JoinStage, IntoBson};
use connector_interface::{
    AggregationFilter, CompositeCondition, CompositeFilter, Filter, OneRelationIsNullFilter, QueryMode,
    RelationCondition, RelationFilter, ScalarCompare, ScalarCondition, ScalarFilter, ScalarListFilter,
    ScalarProjection,
};
use mongodb::bson::{doc, Bson, Document, Regex};
use prisma_models::{CompositeFieldRef, PrismaValue, ScalarFieldRef};

#[derive(Debug)]
pub(crate) enum MongoFilter {
    Scalar(Document),
    Composite(Document),
    Relation(MongoRelationFilter),
}

impl MongoFilter {
    pub(crate) fn render(self) -> (Document, Vec<JoinStage>) {
        match self {
            Self::Scalar(document) => (document, vec![]),
            Self::Relation(rf) => (rf.filter, rf.joins),
            Self::Composite(document) => (document, vec![]),
        }
    }

    pub(crate) fn relation(filter: Document, joins: Vec<JoinStage>) -> Self {
        Self::Relation(MongoRelationFilter { filter, joins })
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
pub(crate) fn convert_filter(
    filter: Filter,
    invert: bool,
    is_having_filter: bool,
    prefix: FilterPrefix,
) -> crate::Result<MongoFilter> {
    let filter = fold_compounds(filter);
    let filter_pair = match filter {
        Filter::And(filters) if invert => coerce_empty(false, "$or", filters, invert, is_having_filter, prefix)?,
        Filter::And(filters) => coerce_empty(true, "$and", filters, invert, is_having_filter, prefix)?,

        Filter::Or(filters) if invert => coerce_empty(true, "$and", filters, invert, is_having_filter, prefix)?,
        Filter::Or(filters) => coerce_empty(false, "$or", filters, invert, is_having_filter, prefix)?,

        Filter::Not(filters) if invert => coerce_empty(false, "$or", filters, !invert, is_having_filter, prefix)?,
        Filter::Not(filters) => coerce_empty(true, "$and", filters, !invert, is_having_filter, prefix)?,

        Filter::Scalar(sf) => scalar_filter(sf, invert, true, is_having_filter, prefix)?,
        Filter::Empty => MongoFilter::Scalar(doc! {}),
        Filter::ScalarList(slf) => scalar_list_filter(slf, invert, prefix)?,
        Filter::OneRelationIsNull(filter) => one_is_null(filter, invert),
        Filter::Relation(rfilter) => relation_filter(rfilter, invert, prefix)?,
        Filter::Aggregation(filter) => aggregation_filter(filter, invert, prefix)?,
        Filter::Composite(filter) => composite_filter(filter, invert, prefix)?,
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
    is_having_filter: bool,
    prefix: FilterPrefix,
) -> crate::Result<MongoFilter> {
    if filters.is_empty() {
        // We need to create a truthy or falsey expression for empty filter queries, e.g. AND / OR / NOT.
        // We abuse the fact that we can create an always failing or succeeding condition with logical `and` and `or` operators,
        // for example "a field exists or doesn't exist" is always true, "a field exists and doesn't exist" is always false.
        let doc = if truthy {
            doc! { "$or": [ { "__prisma_marker": { "$exists": 1 }}, { "__prisma_marker": { "$exists": 0 }} ] }
        } else {
            doc! { "$and": [ { "__prisma_marker": { "$exists": 1 }}, { "__prisma_marker": { "$exists": 0 }} ] }
        };

        Ok(MongoFilter::Scalar(doc))
    } else {
        fold_filters(operation, filters, invert, is_having_filter, prefix)
    }
}

fn fold_filters(
    operation: &str,
    filters: Vec<Filter>,
    invert: bool,
    is_having_filter: bool,
    prefix: FilterPrefix,
) -> crate::Result<MongoFilter> {
    let filters = filters
        .into_iter()
        .map(|f| Ok(convert_filter(f, invert, is_having_filter, prefix.clone())?.render()))
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
    include_field_wrapper: bool,
    is_having_filter: bool,
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
        QueryMode::Default => default_scalar_filter(&field, filter.condition.invert(invert))?,
        QueryMode::Insensitive => insensitive_scalar_filter(&field, filter.condition.invert(invert))?,
    };

    // Explanation: Having filters can only appear in group by queries.
    // All group by fields go into the _id key of the result document.
    // As it is the only point where the flat scalars are contained for the group,
    // we need to refer to the object.
    let field_name = match is_having_filter {
        true => format!("_id.{}", field.db_name()),
        false => field.db_name().to_string(),
    };

    let field_name = prefix.render_with(field_name);

    if include_field_wrapper {
        Ok(MongoFilter::Scalar(doc! { field_name: filter }))
    } else {
        Ok(MongoFilter::Scalar(filter))
    }
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
        ScalarCondition::JsonCompare(jc) => match *jc.condition {
            ScalarCondition::Equals(value) => {
                let bson = (field, value).into_bson()?;
                doc! { "$eq": bson }
            }
            ScalarCondition::NotEquals(value) => {
                let bson = (field, value).into_bson()?;
                doc! { "$ne": bson }
            }
            _ => unimplemented!("Only equality JSON filtering is supported on MongoDB."),
        },
        ScalarCondition::Search(_, _) => unimplemented!("Full-text search is not supported yet on MongoDB"),
        ScalarCondition::NotSearch(_, _) => unimplemented!("Full-text search is not supported yet on MongoDB"),
    })
}

/// Insensitive filters are only reachable with TypeIdentifier::String (or UUID, which is string as well for us).
fn insensitive_scalar_filter(field: &ScalarFieldRef, condition: ScalarCondition) -> crate::Result<Document> {
    match condition {
        ScalarCondition::Equals(val) => Ok(doc! { "$regex": to_regex(field, "^", val, "$", true)? }),
        ScalarCondition::NotEquals(val) => Ok(doc! { "$not": { "$regex": to_regex(field, "^", val, "$", true)? }}),

        ScalarCondition::Contains(val) => Ok(doc! { "$regex": to_regex(field, ".*", val, ".*", true)? }),
        ScalarCondition::NotContains(val) => Ok(doc! { "$not": { "$regex": to_regex(field, ".*", val, ".*", true)? }}),
        ScalarCondition::StartsWith(val) => Ok(doc! { "$regex": to_regex(field, "^", val, "", true)?  }),
        ScalarCondition::NotStartsWith(val) => Ok(doc! { "$not": { "$regex": to_regex(field, "^", val, "", true)? }}),
        ScalarCondition::EndsWith(val) => Ok(doc! { "$regex": to_regex(field, "", val, "$", true)? }),
        ScalarCondition::NotEndsWith(val) => Ok(doc! { "$not": { "$regex": to_regex(field, "", val, "$", true)? }}),
        ScalarCondition::LessThan(val) => Ok(doc! { "$lt": (field, val).into_bson()? }),
        ScalarCondition::LessThanOrEquals(val) => Ok(doc! { "$lte": (field, val).into_bson()? }),
        ScalarCondition::GreaterThan(val) => Ok(doc! { "$gt": (field, val).into_bson()? }),
        ScalarCondition::GreaterThanOrEquals(val) => Ok(doc! { "$gte": (field, val).into_bson()? }),
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

                Ok(doc! { "$in": bson_values })
            }

            _ => Ok(doc! { "$in": to_regex_list(field, "^", vals, "$", true)? }),
        },
        ScalarCondition::NotIn(vals) => Ok(doc! { "$nin": to_regex_list(field, "^", vals, "$", true)? }),
        ScalarCondition::JsonCompare(_) => Err(MongoError::Unsupported(
            "JSON filtering is not yet supported on MongoDB".to_string(),
        )),
        ScalarCondition::Search(_, _) | ScalarCondition::NotSearch(_, _) => Err(MongoError::Unsupported(
            "Full-text search is not supported yet on MongoDB".to_string(),
        )),
    }
}

/// Filters available on list fields.
fn scalar_list_filter(filter: ScalarListFilter, invert: bool, prefix: FilterPrefix) -> crate::Result<MongoFilter> {
    let field = filter.field;
    let prefixed = prefix.render_with(field.db_name().into());

    // Of course Mongo needs special filters for the inverted case, everything else would be too easy.
    let filter_doc = if invert {
        match filter.condition {
            // "Contains element" -> "Does not contain element"
            connector_interface::ScalarListCondition::Contains(val) => {
                doc! { prefixed: { "$elemMatch": { "$not": { "$eq": (&field, val).into_bson()? }}}}
            }

            // "Contains all elements" -> "Does not contain any of the elements"
            connector_interface::ScalarListCondition::ContainsEvery(vals) => {
                doc! { prefixed: { "$nin": (&field, PrismaValue::List(vals)).into_bson()? }}
            }

            // "Contains some of the elements" -> "Does not contain some of the elements"
            connector_interface::ScalarListCondition::ContainsSome(vals) => {
                doc! { prefixed: { "$elemMatch": { "$not": { "$in": (&field, PrismaValue::List(vals)).into_bson()? }}}}
            }

            // Empty -> not empty and vice versa
            connector_interface::ScalarListCondition::IsEmpty(check_for_empty) => {
                if check_for_empty && !invert {
                    doc! { prefixed: { "$size": 0 }}
                } else {
                    doc! { prefixed: { "$not": { "$size": 0 }}}
                }
            }
        }
    } else {
        match filter.condition {
            connector_interface::ScalarListCondition::Contains(val) => {
                doc! { prefixed: (&field, val).into_bson()? }
            }

            connector_interface::ScalarListCondition::ContainsEvery(vals) if vals.is_empty() => {
                // Empty hasEvery: Return all records.
                doc! { "_id": { "$exists": 1 }}
            }

            connector_interface::ScalarListCondition::ContainsEvery(vals) => {
                doc! { prefixed: { "$all": (&field, PrismaValue::List(vals)).into_bson()? }}
            }

            connector_interface::ScalarListCondition::ContainsSome(vals) if vals.is_empty() => {
                // Empty hasSome: Return no records.
                doc! { "_id": { "$exists": 0 }}
            }

            connector_interface::ScalarListCondition::ContainsSome(vals) => {
                doc! { "$or": vals.into_iter().map(|val| Ok(doc! { prefixed.clone(): (&field, val).into_bson()? }) ).collect::<crate::Result<Vec<_>>>()?}
            }

            connector_interface::ScalarListCondition::IsEmpty(empty) => {
                if empty {
                    doc! { prefixed: { "$size": 0 }}
                } else {
                    doc! { prefixed: { "$not": { "$size": 0 }}}
                }
            }
        }
    };

    Ok(MongoFilter::Scalar(filter_doc))
}

// Can be optimized by checking inlined fields on the left side instead of always joining.
fn one_is_null(filter: OneRelationIsNullFilter, invert: bool) -> MongoFilter {
    let rf = filter.field;
    let relation_name = &rf.relation().name;
    let join_stage = JoinStage::new(rf);

    let filter_doc = if invert {
        doc! { relation_name: { "$not": { "$size": 0 }}}
    } else {
        doc! { relation_name: { "$size": 0 }}
    };

    MongoFilter::relation(filter_doc, vec![join_stage])
}

/// Builds a Mongo relation filter depth-first.
fn relation_filter(filter: RelationFilter, invert: bool, prefix: FilterPrefix) -> crate::Result<MongoFilter> {
    let from_field = filter.field;
    let relation_name = &from_field.relation().name;
    let nested_filter = *filter.nested_filter;

    // Tmp condition check while mongo is getting fully tested.
    let is_empty = matches!(nested_filter, Filter::Empty);

    // EveryRelatedRecord requires an inherent invert for Mongo.
    let (nested_filter, nested_joins) = convert_filter(
        nested_filter,
        matches!(&filter.condition, RelationCondition::EveryRelatedRecord),
        false,
        prefix,
    )?
    .render();

    let mut join_stage = JoinStage::new(from_field);
    join_stage.extend_nested(nested_joins);

    let filter_doc = match filter.condition {
        connector_interface::RelationCondition::EveryRelatedRecord => {
            if is_empty {
                doc! { "$not": { "$all": [{ "$elemMatch": { "_id": { "$exists": 0 }} }] }}
            } else {
                doc! { "$not": { "$all": [{ "$elemMatch": nested_filter }] }}
            }
        }
        connector_interface::RelationCondition::AtLeastOneRelatedRecord => {
            doc! { "$elemMatch": nested_filter }
        }
        connector_interface::RelationCondition::NoRelatedRecord => {
            if is_empty {
                doc! { "$size": 0 }
            } else {
                doc! { "$not": { "$all": [{ "$elemMatch": nested_filter }] }}
            }
        }
        connector_interface::RelationCondition::ToOneRelatedRecord => {
            doc! { "$all": [{ "$elemMatch": nested_filter }]}
        }
    };

    if invert {
        Ok(MongoFilter::relation(
            doc! { relation_name: { "$not": filter_doc }},
            vec![join_stage],
        ))
    } else {
        Ok(MongoFilter::relation(
            doc! { relation_name: filter_doc },
            vec![join_stage],
        ))
    }
}

fn aggregation_filter(filter: AggregationFilter, invert: bool, prefix: FilterPrefix) -> crate::Result<MongoFilter> {
    match filter {
        AggregationFilter::Count(filter) => aggregate_conditions("count", *filter, invert, prefix),
        AggregationFilter::Average(filter) => aggregate_conditions("avg", *filter, invert, prefix),
        AggregationFilter::Sum(filter) => aggregate_conditions("sum", *filter, invert, prefix),
        AggregationFilter::Min(filter) => aggregate_conditions("min", *filter, invert, prefix),
        AggregationFilter::Max(filter) => aggregate_conditions("max", *filter, invert, prefix),
    }
}

fn aggregate_conditions(op: &str, filter: Filter, invert: bool, prefix: FilterPrefix) -> crate::Result<MongoFilter> {
    let sf = match filter {
        Filter::Scalar(sf) => sf,
        _ => unimplemented!(),
    };

    let field = match &sf.projection {
        ScalarProjection::Compound(_) => {
            unimplemented!("Compound aggregate projections are unsupported.")
        }

        ScalarProjection::Single(field) => field.clone(),
    };

    let (filter, _) = scalar_filter(sf, invert, false, false, prefix)?.render();

    Ok(MongoFilter::Scalar(
        doc! { format!("{}_{}", op, field.db_name()): filter },
    ))
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

fn composite_filter(filter: CompositeFilter, invert: bool, prefix: FilterPrefix) -> crate::Result<MongoFilter> {
    let field = filter.field;

    let filter_doc = match *filter.condition {
        CompositeCondition::Every(filter) => {
            let is_empty = matches!(filter, Filter::Empty);

            // `Every` filter requires inherent invert because of how the filters are implemented on Mongo.
            let (nested_filter, _) = convert_filter(filter, true, false, FilterPrefix::default())?.render();

            if is_empty {
                doc! { "$not": { "$all": [{ "$elemMatch": { "__prisma_truthy_marker": { "$exists": 1 }} }] }}
            } else {
                doc! { "$not": { "$all": [{ "$elemMatch": nested_filter }] }}
            }
        }

        CompositeCondition::Some(filter) => {
            let (nested_filter, _) = convert_filter(filter, false, false, FilterPrefix::default())?.render();
            doc! { "$elemMatch": nested_filter }
        }

        CompositeCondition::None(filter) => {
            let (nested_filter, _) = convert_filter(filter, false, false, FilterPrefix::default())?.render();

            doc! { "$not": { "$all": [{ "$elemMatch": nested_filter }] }}
        }

        CompositeCondition::Equals(value) => {
            doc! { "$eq": (&field, value).into_bson()? }
        }

        CompositeCondition::Empty(should_be_empty) => {
            if should_be_empty {
                doc! { "$size": 0 }
            } else {
                doc! { "$not": { "$size": 0 }}
            }
        }

        CompositeCondition::Is(filter) => {
            let (nested_filter, _) =
                convert_filter(filter, invert, false, prefix.append_cloned(field.db_name()))?.render();

            return Ok(MongoFilter::Composite(nested_filter));
        }

        CompositeCondition::IsNot(filter) => {
            let (nested_filter, _) =
                convert_filter(filter, !invert, false, prefix.append_cloned(field.db_name()))?.render();

            return Ok(MongoFilter::Composite(nested_filter));
        }
    };

    let field_filter_name = prefix.render_with(field.db_name().into());

    if invert {
        Ok(MongoFilter::Composite(
            doc! { field_filter_name: { "$not": filter_doc }},
        ))
    } else {
        Ok(MongoFilter::Composite(doc! { field_filter_name: filter_doc }))
    }
}

#[derive(Clone, Default)]
pub(crate) struct FilterPrefix {
    parts: Vec<String>,
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
        if self.parts.is_empty() {
            target
        } else {
            format!("{}.{}", self.render(), target)
        }
    }
}

impl From<&CompositeFieldRef> for FilterPrefix {
    fn from(cf: &CompositeFieldRef) -> Self {
        Self {
            parts: vec![cf.db_name().to_owned()],
        }
    }
}
