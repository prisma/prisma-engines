use crate::IntoBson;
use connector_interface::{Filter, QueryMode, ScalarCondition, ScalarFilter, ScalarListFilter};
use mongodb::bson::{doc, Bson, Document, Regex};
use prisma_models::{PrismaValue, ScalarFieldRef};
use std::unimplemented;

// Mongo filters are a BSON document.
impl IntoBson for Filter {
    fn into_bson(self) -> crate::Result<Bson> {
        match self {
            Filter::And(filters) => {
                let filters = filters
                    .into_iter()
                    .map(|f| f.into_bson())
                    .collect::<crate::Result<Vec<_>>>()?;

                Ok(doc! { "$and": Bson::Array(filters) }.into())
            }

            Filter::Or(filters) => {
                let filters = filters
                    .into_iter()
                    .map(|f| f.into_bson())
                    .collect::<crate::Result<Vec<_>>>()?;

                Ok(doc! { "$or": Bson::Array(filters) }.into())
            }

            Filter::Not(filters) => {
                let filters = filters
                    .into_iter()
                    .map(|f| f.into_bson())
                    .collect::<crate::Result<Vec<_>>>()?;

                Ok(doc! { "$not": Bson::Array(filters) }.into())
            }

            Filter::Scalar(sf) => sf.into_bson(),
            Filter::Empty => Ok(Document::new().into()),
            Filter::ScalarList(slf) => slf.into_bson(),
            // Filter::OneRelationIsNull(_) => {}
            // Filter::Relation(_) => {}
            // Filter::BoolFilter(b) => {} // Potentially not doable.
            // Filter::Aggregation(_) => {}
            _ => todo!("Incomplete filter implementation."),
        }
    }
}

impl IntoBson for ScalarFilter {
    fn into_bson(self) -> crate::Result<Bson> {
        // Todo: Find out what Compound cases are really. (Guess: Relation fields with multi-field FK?)
        let field = match self.projection {
            connector_interface::ScalarProjection::Single(sf) => sf,
            connector_interface::ScalarProjection::Compound(mut c) if c.len() == 1 => c.pop().unwrap(),
            connector_interface::ScalarProjection::Compound(_) => unimplemented!("Compound filter case."),
        };

        let filter = match self.mode {
            QueryMode::Default => default_scalar_filter(&field, self.condition)?,
            QueryMode::Insensitive => insensitive_scalar_filter(&field, self.condition)?,
        };

        Ok(doc! { field.db_name(): filter }.into())
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
            _ => doc! { "$in": PrismaValue::List(vals).into_bson()? },
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

impl IntoBson for ScalarListFilter {
    fn into_bson(self) -> crate::Result<Bson> {
        let field = self.field;

        let filter_doc = match self.condition {
            connector_interface::ScalarListCondition::Contains(val) => {
                doc! { field.db_name(): val.into_bson()? }
            }
            connector_interface::ScalarListCondition::ContainsEvery(vals) => {
                doc! { field.db_name(): { "$all": PrismaValue::List(vals).into_bson()? }}
            }
            connector_interface::ScalarListCondition::ContainsSome(vals) => {
                doc! { "$or": vals.into_iter().map(|val| Ok(doc! { field.db_name(): val.into_bson()? }) ).collect::<crate::Result<Vec<_>>>()?}
            }
            connector_interface::ScalarListCondition::IsEmpty(empty) => {
                if empty {
                    doc! { field.db_name(): { "$size": 0 }}
                } else {
                    doc! { field.db_name(): { "$not": { "$size": 0 }}}
                }
            }
        };

        Ok(filter_doc.into())
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
