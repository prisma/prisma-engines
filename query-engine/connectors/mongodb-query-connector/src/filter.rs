use crate::{constants::group_by, error::MongoError, join::JoinStage, query_builder::AggregationType, IntoBson};
use mongodb::bson::{doc, Bson, Document};
use query_structure::*;

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

pub(crate) struct MongoFilterVisitor {
    /// The prefix that's applied to the field name for which we render the filter.
    prefix: FilterPrefix,
    /// An optional custom prefix for referenced fields.
    /// By default, referenced fields will use the same prefix as the field on which a filter is applied.
    /// For some edge-cases like aggregations though, those prefixes need to be different.
    ref_prefix: Option<FilterPrefix>,
    /// Whether the filter should be converted inverted.
    invert: bool,
    /// Whether the undefined exclusion should be inverted.
    invert_undefined_exclusion: bool,
    /// Whether the parent filter is an aggregation filter.
    /// If so, specifies which type of aggregation filter it is.
    parent_aggregation_type: Option<AggregationType>,
}

impl MongoFilterVisitor {
    pub(crate) fn new(prefix: impl Into<FilterPrefix>, invert: bool) -> Self {
        Self {
            prefix: prefix.into(),
            ref_prefix: None,
            invert,
            invert_undefined_exclusion: false,
            parent_aggregation_type: None,
        }
    }

    /// Builds a MongoDB query filter from a Prisma filter.
    pub(crate) fn visit(&mut self, filter: Filter) -> crate::Result<MongoFilter> {
        self.visit_filter(filter)
    }

    fn visit_filter(&mut self, filter: Filter) -> crate::Result<MongoFilter> {
        let filter = fold_compounds(filter);

        let filter_pair = match filter {
            Filter::And(filters) if self.invert() => self.visit_boolean_operator("$or", filters, false)?,
            Filter::And(filters) => self.visit_boolean_operator("$and", filters, true)?,

            Filter::Or(filters) if self.invert() => self.visit_boolean_operator("$and", filters, true)?,
            Filter::Or(filters) => self.visit_boolean_operator("$or", filters, false)?,

            Filter::Not(filters) if self.invert() => {
                self.flip_invert();
                let result = self.visit_boolean_operator("$or", filters, false)?;
                self.flip_invert();
                result
            }
            Filter::Not(filters) => {
                self.flip_invert();
                let result = self.visit_boolean_operator("$and", filters, true)?;
                self.flip_invert();
                result
            }
            Filter::Scalar(sf) => self.visit_scalar_filter(sf)?,
            Filter::Empty => MongoFilter::Scalar(doc! {}),
            Filter::ScalarList(slf) => self.visit_scalar_list_filter(slf)?,
            Filter::OneRelationIsNull(filter) => self.visit_one_is_null(filter)?,
            Filter::Relation(rfilter) => self.visit_relation_filter(rfilter.invert(self.invert()))?,
            Filter::Aggregation(filter) => self.visit_aggregation_filter(filter)?,
            Filter::Composite(filter) => self.visit_composite_filter(filter)?,
            Filter::BoolFilter(_) => unimplemented!("MongoDB boolean filter."),
        };

        Ok(filter_pair)
    }

    fn visit_boolean_operator(
        &mut self,
        operation: &str,
        filters: Vec<Filter>,
        truthy: bool,
    ) -> crate::Result<MongoFilter> {
        if filters.is_empty() {
            // We need to create a truthy or falsey expression for empty filter queries, e.g. AND / OR / NOT.
            // We abuse the fact that we can create an always failing or succeeding condition with logical `and` and `or` operators,
            // for example "a field exists or doesn't exist" is always true, "a field exists and doesn't exist" is always false.
            let stub_condition = render_stub_condition(truthy);

            Ok(MongoFilter::Scalar(stub_condition))
        } else {
            self.fold_filters(operation, filters)
        }
    }

    fn fold_filters(&mut self, operation: &str, filters: Vec<Filter>) -> crate::Result<MongoFilter> {
        let filters = filters
            .into_iter()
            .map(|f| Ok(self.visit(f)?.render()))
            .collect::<crate::Result<Vec<_>>>()?;

        let (filters, joins) = fold_nested(filters);
        let filter_doc = doc! { operation: filters };

        Ok(MongoFilter::relation(filter_doc, joins))
    }

    fn visit_scalar_filter(&self, filter: ScalarFilter) -> crate::Result<MongoFilter> {
        let field = match filter.projection {
            ScalarProjection::Single(sf) => sf,
            ScalarProjection::Compound(mut c) if c.len() == 1 => c.pop().unwrap(),
            ScalarProjection::Compound(_) => {
                unreachable!(
                    "Multi-field compound filter case hit when it should have been folded into normal filters previously."
                )
            }
        };

        let filter = match filter.mode {
            QueryMode::Default => self.default_scalar_filter(&field, filter.condition.invert(self.invert()))?,
            QueryMode::Insensitive => self.insensitive_scalar_filter(&field, filter.condition.invert(self.invert()))?,
        };

        Ok(MongoFilter::Scalar(filter))
    }

    // Note contains / startsWith / endsWith are only applicable to String types in the schema.
    fn default_scalar_filter(&self, field: &ScalarFieldRef, condition: ScalarCondition) -> crate::Result<Document> {
        let field_name = (self.prefix(), field).into_bson()?;
        let field_ref = condition.as_field_ref().cloned();
        let is_set_cond = matches!(&condition, ScalarCondition::IsSet(_));

        let filter_doc = match condition {
            ScalarCondition::Equals(val) => {
                doc! { "$eq": [&field_name, self.coerce_to_bson_for_filter(field, val)?] }
            }
            ScalarCondition::NotEquals(val) => {
                doc! { "$ne": [&field_name, self.coerce_to_bson_for_filter(field, val)?] }
            }
            ScalarCondition::Contains(val) => self.regex_match(&field_name, field, ".*", val, ".*", false)?,
            ScalarCondition::NotContains(val) => {
                doc! { "$not": self.regex_match(&field_name, field, ".*", val, ".*", false)? }
            }
            ScalarCondition::StartsWith(val) => self.regex_match(&field_name, field, "^", val, "", false)?,
            ScalarCondition::NotStartsWith(val) => {
                doc! { "$not": self.regex_match(&field_name, field, "^", val, "", false)? }
            }
            ScalarCondition::EndsWith(val) => self.regex_match(&field_name, field, "", val, "$", false)?,
            ScalarCondition::NotEndsWith(val) => {
                doc! { "$not": self.regex_match(&field_name, field, "", val, "$", false)? }
            }
            ScalarCondition::LessThan(val) => {
                doc! { "$lt": [&field_name, self.coerce_to_bson_for_filter(field, val)?] }
            }
            ScalarCondition::LessThanOrEquals(val) => {
                doc! { "$lte": [&field_name, self.coerce_to_bson_for_filter(field, val)?] }
            }
            ScalarCondition::GreaterThan(val) => {
                doc! { "$gt": [&field_name, self.coerce_to_bson_for_filter(field, val)?] }
            }
            ScalarCondition::GreaterThanOrEquals(val) => {
                doc! { "$gte": [&field_name, self.coerce_to_bson_for_filter(field, val)?] }
            }
            // Todo: The nested list unpack looks like a bug somewhere.
            //       Likely join code mistakenly repacks a list into a list of PrismaValue somewhere in the core.
            ScalarCondition::In(vals) => match vals {
                ConditionListValue::List(values) => {
                    let mut equalities = Vec::with_capacity(values.len());

                    for value in values {
                        // List is list of lists, we need to flatten.
                        // This flattening behaviour does not affect user queries because Prisma does
                        // not support storing arrays as values inside a field. It is possible to have
                        // a 1-dimensional array field, but not 2 dimensional. Thus, we never have
                        // user queries which have arrays in the argument of `in` operator. If we
                        // encounter such case, then this query was produced internally and we can
                        // safely flatten it.
                        if let PrismaValue::List(list) = value {
                            equalities.extend(
                                list.into_iter()
                                    .map(|value| {
                                        let value = self.coerce_to_bson_for_filter(field, value)?;
                                        Ok(doc! { "$eq": [&field_name, value] })
                                    })
                                    .collect::<crate::Result<Vec<_>>>()?,
                            );
                        } else {
                            let value = self.coerce_to_bson_for_filter(field, value)?;
                            equalities.push(doc! { "$eq": [&field_name, value] })
                        }
                    }

                    // Previously, `$in` operator was used instead of a tree of `$or` + `$eq` operators.
                    // At the moment of writing, MongoDB does not optimise aggregation version of `$in`
                    // operator to use indexes, leading to significant performance problems. Until this
                    // is fixed, we rely on `$eq` operator which does have index optimisation implemented.
                    doc! { "$or": equalities }
                }
                ConditionListValue::FieldRef(field_ref) => {
                    // In this context, `field_ref` refers to an array field, so we actually need an `$in` operator.
                    doc! { "$in": [&field_name, coerce_as_array(self.prefixed_field_ref(&field_ref)?)] }
                }
            },
            ScalarCondition::NotIn(vals) => match vals {
                ConditionListValue::List(vals) => {
                    let equalities = vals
                        .into_iter()
                        .map(|value| {
                            let value = self.coerce_to_bson_for_filter(field, value)?;
                            Ok(doc! { "$ne": [&field_name, value] })
                        })
                        .collect::<crate::Result<Vec<_>>>()?;

                    // Previously, `$not` + `$in` operators were used instead of a tree of `$and` + `$ne` operators.
                    // At the moment of writing, MongoDB does not optimise aggregation version of `$in`
                    // operator to use indexes, leading to significant performance problems. Until this
                    // is fixed, we rely on `$ne` operator which does have index optimisation implemented.
                    doc! { "$and": equalities }
                }
                ConditionListValue::FieldRef(field_ref) => {
                    // In this context, `field_ref` refers to an array field, so we actually need an `$in` operator.
                    doc! { "$not": { "$in": [&field_name, coerce_as_array(self.prefixed_field_ref(&field_ref)?)] } }
                }
            },
            ScalarCondition::JsonCompare(jc) => match *jc.condition {
                ScalarCondition::Equals(value) => {
                    let bson = match value {
                        ConditionValue::Value(value) => (field, value).into_bson()?,
                        ConditionValue::FieldRef(field_ref) => self.prefixed_field_ref(&field_ref)?,
                    };

                    doc! { "$eq": [&field_name, bson] }
                }
                ScalarCondition::NotEquals(value) => {
                    let bson = match value {
                        ConditionValue::Value(value) => (field, value).into_bson()?,
                        ConditionValue::FieldRef(field_ref) => self.prefixed_field_ref(&field_ref)?,
                    };

                    doc! { "$ne": [&field_name, bson] }
                }
                _ => unimplemented!("Only equality JSON filtering is supported on MongoDB."),
            },
            ScalarCondition::IsSet(is_set) => render_is_set(&field_name, is_set),
            ScalarCondition::Search(_, _) => unimplemented!("Full-text search is not supported yet on MongoDB"),
            ScalarCondition::NotSearch(_, _) => unimplemented!("Full-text search is not supported yet on MongoDB"),
        };

        let filter_doc = if !is_set_cond {
            exclude_undefineds(&field_name, self.invert_undefined_exclusion(), filter_doc)
        } else {
            filter_doc
        };

        let filter_doc = if let Some(field_ref) = &field_ref {
            exclude_undefineds(
                self.prefixed_field_ref(field_ref)?,
                self.invert_undefined_exclusion(),
                filter_doc,
            )
        } else {
            filter_doc
        };

        Ok(filter_doc)
    }

    /// Insensitive filters are only reachable with TypeIdentifier::String (or UUID, which is string as well for us).
    fn insensitive_scalar_filter(&self, field: &ScalarFieldRef, condition: ScalarCondition) -> crate::Result<Document> {
        let field_name = (self.prefix(), field).into_bson()?;
        let field_ref = condition.as_field_ref().cloned();
        let is_set_cond = matches!(&condition, ScalarCondition::IsSet(_));

        let filter_doc = match condition {
            ScalarCondition::Equals(val) => self.regex_match(&field_name, field, "^", val, "$", true),
            ScalarCondition::NotEquals(val) => {
                Ok(doc! { "$not": self.regex_match(&field_name, field, "^", val, "$", true)? })
            }

            ScalarCondition::Contains(val) => self.regex_match(&field_name, field, ".*", val, ".*", true),
            ScalarCondition::NotContains(val) => {
                Ok(doc! { "$not": self.regex_match(&field_name, field, ".*", val, ".*", true)?})
            }
            ScalarCondition::StartsWith(val) => self.regex_match(&field_name, field, "^", val, "", true),
            ScalarCondition::NotStartsWith(val) => {
                Ok(doc! { "$not": self.regex_match(&field_name, field, "^", val, "", true)? })
            }
            ScalarCondition::EndsWith(val) => self.regex_match(&field_name, field, "", val, "$", true),
            ScalarCondition::NotEndsWith(val) => {
                Ok(doc! { "$not": self.regex_match(&field_name, field, "", val, "$", true)? })
            }
            ScalarCondition::LessThan(val) => {
                let bson = match val {
                    ConditionValue::Value(value) => (field, value).into_bson()?,
                    ConditionValue::FieldRef(field_ref) => self.prefixed_field_ref(&field_ref)?,
                };

                Ok(doc! { "$lt": [{ "$toLower": &field_name }, { "$toLower": bson }] })
            }
            ScalarCondition::LessThanOrEquals(val) => {
                let bson = match val {
                    ConditionValue::Value(value) => (field, value).into_bson()?,
                    ConditionValue::FieldRef(field_ref) => self.prefixed_field_ref(&field_ref)?,
                };

                Ok(doc! { "$lte": [{ "$toLower": &field_name }, { "$toLower": bson }] })
            }
            ScalarCondition::GreaterThan(val) => {
                let bson = match val {
                    ConditionValue::Value(value) => (field, value).into_bson()?,
                    ConditionValue::FieldRef(field_ref) => self.prefixed_field_ref(&field_ref)?,
                };

                Ok(doc! { "$gt": [{ "$toLower": &field_name }, { "$toLower": bson }] })
            }
            ScalarCondition::GreaterThanOrEquals(val) => {
                let bson = match val {
                    ConditionValue::Value(value) => (field, value).into_bson()?,
                    ConditionValue::FieldRef(field_ref) => self.prefixed_field_ref(&field_ref)?,
                };

                Ok(doc! { "$gte": [{ "$toLower": &field_name }, { "$toLower": bson }] })
            }
            // Todo: The nested list unpack looks like a bug somewhere.
            // Likely join code mistakenly repacks a list into a list of PrismaValue somewhere in the core.
            ScalarCondition::In(vals) => match vals {
                ConditionListValue::List(vals) => match vals.split_first() {
                    // List is list of lists, we need to flatten.
                    Some((PrismaValue::List(_), _)) => {
                        let mut matches = Vec::with_capacity(vals.len());

                        for pv in vals {
                            if let PrismaValue::List(inner) = pv {
                                for val in inner {
                                    matches.push(self.regex_match(&field_name, field, "^", val, "$", true)?)
                                }
                            }
                        }

                        Ok(doc! { "$or": matches })
                    }

                    _ => {
                        let matches = vals
                            .into_iter()
                            .map(|val| self.regex_match(&field_name, field, "^", val, "$", true))
                            .collect::<crate::Result<Vec<_>>>()?;

                        Ok(doc! { "$or": matches })
                    }
                },
                ConditionListValue::FieldRef(field_ref) => Ok(render_some(
                    (self.prefix(), &field_ref).into_bson()?,
                    "elem",
                    self.regex_match(&Bson::from("$$elem"), field, "^", field, "$", true)?,
                    true,
                )),
            },
            ScalarCondition::NotIn(vals) => match vals {
                ConditionListValue::List(vals) => {
                    let matches = vals
                        .into_iter()
                        .map(|val| {
                            self.regex_match(&field_name, field, "^", val, "$", true)
                                .map(|rgx_doc| doc! { "$not": rgx_doc })
                        })
                        .collect::<crate::Result<Vec<_>>>()?;

                    Ok(doc! { "$and": matches })
                }
                ConditionListValue::FieldRef(field_ref) => Ok(render_every(
                    (self.prefix(), &field_ref).into_bson()?,
                    "elem",
                    self.regex_match(&Bson::from("$$elem"), field, "^", field, "$", true)
                        .map(|rgx_doc| doc! { "$not": rgx_doc })?,
                    true,
                )),
            },
            ScalarCondition::IsSet(is_set) => Ok(render_is_set(&field_name, is_set)),
            ScalarCondition::JsonCompare(_) => Err(MongoError::Unsupported(
                "JSON filtering is not yet supported on MongoDB".to_string(),
            )),
            ScalarCondition::Search(_, _) | ScalarCondition::NotSearch(_, _) => Err(MongoError::Unsupported(
                "Full-text search is not supported yet on MongoDB".to_string(),
            )),
        }?;

        let filter_doc = if !is_set_cond {
            exclude_undefineds(&field_name, self.invert_undefined_exclusion(), filter_doc)
        } else {
            filter_doc
        };

        let filter_doc = if let Some(field_ref) = &field_ref {
            exclude_undefineds(
                self.prefixed_field_ref(field_ref)?,
                self.invert_undefined_exclusion(),
                filter_doc,
            )
        } else {
            filter_doc
        };

        Ok(filter_doc)
    }

    /// Filters available on list fields.
    fn visit_scalar_list_filter(&self, filter: ScalarListFilter) -> crate::Result<MongoFilter> {
        let field = &filter.field;
        let field_name = (self.prefix(), field).into_bson()?;
        let field_ref = filter.as_field_ref().cloned();

        let filter_doc = match filter.condition {
            ScalarListCondition::Contains(val) => {
                let bson = match val {
                    ConditionValue::Value(value) => (field, value).into_bson()?,
                    ConditionValue::FieldRef(field_ref) => self.prefixed_field_ref(&field_ref)?,
                };

                doc! { "$in": [bson, coerce_as_array(&field_name)] }
            }

            ScalarListCondition::ContainsEvery(vals) if vals.is_empty() => {
                // Empty hasEvery: Return all records.
                render_stub_condition(true)
            }
            ScalarListCondition::ContainsEvery(ConditionListValue::List(vals)) => {
                let ins = vals
                    .into_iter()
                    .map(|val| {
                        (field, val)
                            .into_bson()
                            .map(|bson_val| doc! { "$in": [bson_val, coerce_as_array(&field_name)] })
                    })
                    .collect::<crate::Result<Vec<_>>>()?;

                doc! { "$and": ins }
            }
            ScalarListCondition::ContainsEvery(ConditionListValue::FieldRef(field_ref)) => render_every(
                &field_name,
                "elem",
                doc! { "$in": ["$$elem", coerce_as_array((self.prefix(), &field_ref).into_bson()?)] },
                true,
            ),

            ScalarListCondition::ContainsSome(vals) if vals.is_empty() => {
                // Empty hasSome: Return no records.
                render_stub_condition(false)
            }
            ScalarListCondition::ContainsSome(ConditionListValue::List(vals)) => {
                let ins = vals
                    .into_iter()
                    .map(|val| {
                        (field, val)
                            .into_bson()
                            .map(|bson_val| doc! { "$in": [bson_val, coerce_as_array(&field_name)] })
                    })
                    .collect::<crate::Result<Vec<_>>>()?;

                doc! { "$or": ins }
            }
            ScalarListCondition::ContainsSome(ConditionListValue::FieldRef(field_ref)) => render_some(
                &field_name,
                "elem",
                doc! { "$in": ["$$elem", coerce_as_array((self.prefix(), &field_ref).into_bson()?)] },
                true,
            ),

            ScalarListCondition::IsEmpty(true) => {
                doc! { "$eq": [render_size(&field_name, true), 0] }
            }
            ScalarListCondition::IsEmpty(false) => {
                doc! { "$gt": [render_size(&field_name, true), 0] }
            }
        };

        let filter_doc = if self.invert() {
            doc! { "$not": filter_doc }
        } else {
            filter_doc
        };

        let filter_doc = exclude_undefineds(&field_name, self.invert_undefined_exclusion(), filter_doc);

        let filter_doc = if let Some(field_ref) = field_ref.as_ref() {
            exclude_undefineds(
                self.prefixed_field_ref(field_ref)?,
                self.invert_undefined_exclusion(),
                filter_doc,
            )
        } else {
            filter_doc
        };

        Ok(MongoFilter::Scalar(filter_doc))
    }

    fn visit_aggregation_filter(&self, filter: AggregationFilter) -> crate::Result<MongoFilter> {
        match filter {
            AggregationFilter::Count(filter) => self.aggregate_conditions(AggregationType::Count, *filter),
            AggregationFilter::Average(filter) => self.aggregate_conditions(AggregationType::Average, *filter),
            AggregationFilter::Sum(filter) => self.aggregate_conditions(AggregationType::Sum, *filter),
            AggregationFilter::Min(filter) => self.aggregate_conditions(AggregationType::Min, *filter),
            AggregationFilter::Max(filter) => self.aggregate_conditions(AggregationType::Max, *filter),
        }
    }

    fn aggregate_conditions(&self, aggregation_type: AggregationType, filter: Filter) -> crate::Result<MongoFilter> {
        let scalar_filter = filter.into_scalar().unwrap();
        let field = scalar_filter.projection.as_single().unwrap();

        // An aggregation filter can only refer to its aggregated field, which is already the "target".
        // Therefore, we make sure the additional target in `scalar_filter` won't be rendered.
        let prefix = FilterPrefix::from(format!("{}_{}", aggregation_type, field.db_name())).ignore_target(true);

        let (filter, _) = MongoFilterVisitor::new(prefix, self.invert())
            .set_invert_undefined_exclusion(self.invert_undefined_exclusion())
            .set_parent_aggregation_type(aggregation_type)
            // Referenced fields in a having filter _have_ to be _grouped by_. They will therefore be
            // gathered under the UNDERSCORE_ID field and needs to be referenced from there.
            // Have a look at the `GroupByBuilder` responsible for rendering this part of the query.
            .set_ref_prefix(Some(FilterPrefix::from(group_by::UNDERSCORE_ID)))
            .visit_scalar_filter(scalar_filter)?
            .render();

        Ok(MongoFilter::Scalar(filter))
    }

    fn visit_composite_filter(&self, filter: CompositeFilter) -> crate::Result<MongoFilter> {
        let field = filter.field;
        let field_name = (&self.prefix.clone(), &field).into_bson()?;
        let is_set_cond = matches!(*filter.condition, CompositeCondition::IsSet(_));

        let filter_doc = match *filter.condition {
            CompositeCondition::Every(filter) => {
                let (every, _) =
                    render_every_from_filter(&field_name, filter, self.invert_undefined_exclusion(), true)?;

                every
            }

            CompositeCondition::Some(filter) => {
                let (some, _) = render_some_from_filter(&field_name, filter, self.invert_undefined_exclusion(), true)?;

                some
            }

            CompositeCondition::None(filter) => {
                let (none, _) = render_none_from_filter(&field_name, filter, !self.invert_undefined_exclusion(), true)?;

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

                if self.invert() {
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
                let (nested_filter, _) =
                    MongoFilterVisitor::new(self.prefix.append_cloned(field.db_name()), self.invert())
                        .set_invert_undefined_exclusion(self.invert_undefined_exclusion())
                        .visit(filter)?
                        .render();

                return Ok(MongoFilter::Composite(nested_filter));
            }

            CompositeCondition::IsNot(filter) => {
                let (nested_filter, _) =
                    MongoFilterVisitor::new(self.prefix.append_cloned(field.db_name()), !self.invert())
                        .set_invert_undefined_exclusion(self.invert_undefined_exclusion())
                        .visit(filter)?
                        .render();

                return Ok(MongoFilter::Composite(nested_filter));
            }
        };

        let filter_doc = if self.invert() {
            doc! { "$not": filter_doc }
        } else {
            filter_doc
        };

        let filter_doc = if !is_set_cond {
            exclude_undefineds(&field_name, self.invert_undefined_exclusion(), filter_doc)
        } else {
            filter_doc
        };

        Ok(MongoFilter::Composite(filter_doc))
    }

    // Can be optimized by checking inlined fields on the left side instead of always joining.
    fn visit_one_is_null(&self, filter: OneRelationIsNullFilter) -> crate::Result<MongoFilter> {
        let rf = filter.field;
        let field_name = (self.prefix(), &rf).into_bson()?;
        let join_stage = JoinStage::new(rf);

        let filter_doc = if self.invert() {
            doc! { "$gt": [render_size(&field_name, false), 0] }
        } else {
            doc! { "$eq": [render_size(&field_name, false), 0] }
        };

        Ok(MongoFilter::relation(filter_doc, vec![join_stage]))
    }

    /// Builds a Mongo relation filter depth-first.
    fn visit_relation_filter(&self, filter: RelationFilter) -> crate::Result<MongoFilter> {
        let from_field = filter.field;
        let nested_filter = *filter.nested_filter;
        let is_to_one = !from_field.is_list();
        let field_name = (self.prefix(), &from_field).into_bson()?;
        // Tmp condition check while mongo is getting fully tested.
        let is_empty_filter = matches!(nested_filter, Filter::Empty);

        let mut join_stage = JoinStage::new(from_field);

        let filter_doc = match filter.condition {
            RelationCondition::EveryRelatedRecord => {
                let (every, nested_joins) = render_every_from_filter(&field_name, nested_filter, false, false)?;

                join_stage.extend_nested(nested_joins);

                every
            }
            RelationCondition::AtLeastOneRelatedRecord => {
                let (some, nested_joins) = render_some_from_filter(&field_name, nested_filter, false, false)?;

                join_stage.extend_nested(nested_joins);

                some
            }
            RelationCondition::NoRelatedRecord if is_to_one => {
                if is_empty_filter {
                    // Doesn't need coercing the array since joins always return arrays
                    doc! { "$eq": [render_size(&field_name, false), 0] }
                } else {
                    let (none, nested_joins) = render_none_from_filter(&field_name, nested_filter, true, false)?;

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
            RelationCondition::NoRelatedRecord => {
                if is_empty_filter {
                    // Doesn't need coercing the array since joins always return arrays
                    doc! { "$eq": [render_size(&field_name, false), 0] }
                } else {
                    let (none, nested_joins) = render_none_from_filter(&field_name, nested_filter, true, false)?;

                    join_stage.extend_nested(nested_joins);

                    none
                }
            }
            RelationCondition::ToOneRelatedRecord => {
                // To-ones are coerced to single-element arrays via the join.
                // We render an "every" expression on that array to ensure that the predicate is matched.
                let (every, nested_joins) = render_every_from_filter(&field_name, nested_filter, false, false)?;

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

    /// Convert a PrismaValue into Bson, with a special case for `_count` aggregation filter.
    ///
    /// When converting the value of a `_count` aggregation filter for a field that's _not_ numerical,
    /// we force the `TypeIdentifier` to be `Int` to prevent panics.
    fn coerce_to_bson_for_filter(&self, sf: &ScalarFieldRef, value: impl Into<ConditionValue>) -> crate::Result<Bson> {
        match value.into() {
            ConditionValue::Value(value) => {
                if self.parent_is_count_aggregation() && !sf.is_numeric() {
                    (&TypeIdentifier::Int, value).into_bson()
                } else {
                    let bson_value = (sf, value).into_bson()?;
                    Ok(Bson::Document(doc! {"$literal": bson_value}))
                }
            }
            ConditionValue::FieldRef(field_ref) => self.prefixed_field_ref(&field_ref),
        }
    }

    /// Renders a `$regexMatch` expression.
    fn regex_match(
        &self,
        field_name: &Bson,
        field: &ScalarFieldRef,
        prefix: &str,
        value: impl Into<ConditionValue>,
        suffix: &str,
        insensitive: bool,
    ) -> crate::Result<Document> {
        let value: ConditionValue = value.into();
        let options = if insensitive { "i" } else { "" }.to_owned();

        let pattern = match value {
            ConditionValue::Value(value) => Bson::from(format!(
                "{}{}{}",
                prefix,
                (field, value)
                    .into_bson()?
                    .as_str()
                    .expect("Only reachable with String types."),
                suffix
            )),
            ConditionValue::FieldRef(field_ref) => Bson::from(
                doc! { "$concat": [doc! { "$literal": prefix }, (self.ref_prefix(), &field_ref).into_bson()?, doc! { "$literal": suffix }] },
            ),
        };

        Ok(doc! {
            "$regexMatch": {
                "input": field_name,
                "regex": pattern,
                "options": options
            }
        })
    }

    /// Returns the prefix for the field on which a filter is applied.
    fn prefix(&self) -> &FilterPrefix {
        &self.prefix
    }

    fn invert(&self) -> bool {
        self.invert
    }

    fn flip_invert(&mut self) {
        self.invert = !self.invert;
    }

    fn invert_undefined_exclusion(&self) -> bool {
        self.invert_undefined_exclusion
    }

    fn set_invert_undefined_exclusion(mut self, invert: bool) -> Self {
        self.invert_undefined_exclusion = invert;
        self
    }

    fn set_parent_aggregation_type(mut self, aggregation_type: AggregationType) -> Self {
        self.parent_aggregation_type = Some(aggregation_type);
        self
    }

    fn parent_aggregation_type(&self) -> Option<&AggregationType> {
        self.parent_aggregation_type.as_ref()
    }

    fn parent_is_count_aggregation(&self) -> bool {
        matches!(self.parent_aggregation_type(), Some(AggregationType::Count))
    }

    /// Returns the prefix for a referenced field.
    /// If there's no custom `ref_prefix` then use the prefix
    fn ref_prefix(&self) -> &FilterPrefix {
        self.ref_prefix.as_ref().unwrap_or_else(|| self.prefix())
    }

    /// Sets an optional custom prefix for referenced fields.
    /// By default, referenced fields will use the same prefix as the field on which a filter is applied.
    /// For some edge-cases like aggregations though, those prefixes need to be different.
    fn set_ref_prefix(mut self, ref_prefix: Option<FilterPrefix>) -> Self {
        self.ref_prefix = ref_prefix;
        self
    }

    /// Computes the BSON representation of a referenced field.
    fn prefixed_field_ref(&self, field_ref: &ScalarFieldRef) -> crate::Result<Bson> {
        (self.ref_prefix(), field_ref).into_bson()
    }
}

fn fold_compounds(filter: Filter) -> Filter {
    match filter {
        Filter::Scalar(ScalarFilter {
            projection: ScalarProjection::Compound(fields),
            condition: ScalarCondition::In(ConditionListValue::List(value_tuples)),
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

// Todo we should really only join each relation once.
fn fold_nested(nested: Vec<(Document, Vec<JoinStage>)>) -> (Vec<Document>, Vec<JoinStage>) {
    nested.into_iter().fold((vec![], vec![]), |mut acc, next| {
        acc.0.push(next.0);
        acc.1.extend(next.1);
        acc
    })
}

/// Renders a `$size` expression to compute the length of an array.
/// If `coerce_array` is true, the array will be coerced to an empty array in case it's `null` or `undefined`.
fn render_size(field_name: impl Into<Bson>, coerce_array: bool) -> Document {
    if coerce_array {
        doc! { "$size": coerce_as_array(field_name) }
    } else {
        doc! { "$size": field_name.into() }
    }
}

/// Coerces a field to an empty array if it's `null` or `undefined`.
/// Renders an `$ifNull` expression.
fn coerce_as_array(field_name: impl Into<Bson>) -> Document {
    doc! { "$ifNull": [field_name.into(), []] }
}

/// Coerces a field to `null` if it's `null` or `undefined`.
/// Used to convert `undefined` fields to `null`.
/// Renders an `$ifNull` expression.
fn coerce_as_null(field_name: impl Into<Bson>) -> Document {
    doc! { "$ifNull": [field_name.into(), null] }
}

/// Renders an expression that computes whether _some_ of the elements of an array matches the `Filter`.
/// If `coerce_array` is true, the array will be coerced to an empty array in case it's `null` or `undefined`.
fn render_some_from_filter(
    field_name: impl Into<Bson>,
    filter: Filter,
    invert_undefined_exclusion: bool,
    coerce_array: bool,
) -> crate::Result<(Document, Vec<JoinStage>)> {
    // Nested filters needs to be prefixed with `$$elem` so that they refer to the "elem" alias defined in the $filter operator below.
    let prefix = FilterPrefix::from("$elem");
    let (nested_filter, nested_joins) = MongoFilterVisitor::new(prefix, false)
        .set_invert_undefined_exclusion(invert_undefined_exclusion)
        .visit(filter)?
        .render();
    let doc = render_some(field_name, "elem", nested_filter, coerce_array);

    Ok((doc, nested_joins))
}

fn render_some(input: impl Into<Bson>, alias: impl Into<Bson>, cond: impl Into<Bson>, coerce_array: bool) -> Document {
    let input: Bson = if coerce_array {
        coerce_as_array(input).into()
    } else {
        input.into()
    };

    doc! {
      "$gt": [
        {
          "$size": {
            "$filter": {
              "input": input,
              "as": alias.into(),
              "cond": cond.into()
            }
          }
        },
        0
      ]
    }
}

/// Renders an expression that computes whether _all_ of the elements of an array matches the `Filter`.
/// If `coerce_array` is true, the array will be coerced to an empty array in case it's `null` or `undefined`.
fn render_every_from_filter(
    field_name: &Bson,
    filter: Filter,
    invert_undefined_exclusion: bool,
    coerce_array: bool,
) -> crate::Result<(Document, Vec<JoinStage>)> {
    // Nested filters needs to be prefixed with `$$elem` so that they refer to the "elem" alias defined in the $filter operator below.
    let prefix = FilterPrefix::from("$elem");
    let (nested_filter, nested_joins) = MongoFilterVisitor::new(prefix, false)
        .set_invert_undefined_exclusion(invert_undefined_exclusion)
        .visit(filter)?
        .render();
    let doc = render_every(field_name, "elem", nested_filter, coerce_array);

    Ok((doc, nested_joins))
}

fn render_every(input: impl Into<Bson>, alias: impl Into<Bson>, cond: impl Into<Bson>, coerce_array: bool) -> Document {
    let input: Bson = if coerce_array {
        coerce_as_array(input).into()
    } else {
        input.into()
    };

    doc! {
      "$eq": [
        {
          "$size": {
            "$filter": {
              "input": input.clone(),
              "as": alias.into(),
              "cond": cond.into(),
            }
          }
        },
        render_size(input, false)
      ]
    }
}

/// Renders an expression that computes whether _none_ of the elements of an array matches the `Filter`.
/// If `coerce_array` is true, the array will be coerced to an empty array in case it's `null` or `undefined`.
fn render_none_from_filter(
    field_name: &Bson,
    filter: Filter,
    invert_undefined_exclusion: bool,
    coerce_array: bool,
) -> crate::Result<(Document, Vec<JoinStage>)> {
    // Nested filters needs to be prefixed with `$$elem` so that they refer to the "elem" alias defined in the $filter operator below.
    let prefix = FilterPrefix::from("$elem");
    let (nested_filter, nested_joins) = MongoFilterVisitor::new(prefix, false)
        .set_invert_undefined_exclusion(invert_undefined_exclusion)
        .visit(filter)?
        .render();
    let doc = render_none(field_name, "elem", nested_filter, coerce_array);

    Ok((doc, nested_joins))
}

fn render_none(input: impl Into<Bson>, alias: impl Into<Bson>, cond: impl Into<Bson>, coerce_array: bool) -> Document {
    let input: Bson = if coerce_array {
        coerce_as_array(input).into()
    } else {
        input.into()
    };

    doc! {
      "$eq": [
        {
          "$size": {
            "$filter": {
              "input": input,
              "as": alias.into(),
              "cond": cond.into()
            }
          }
        },
        0
      ]
    }
}

/// Renders a stub condition that's either true or false
fn render_stub_condition(truthy: bool) -> Document {
    doc! { "$and": truthy }
}

fn render_is_set(field_name: impl Into<Bson>, is_set: bool) -> Document {
    if is_set {
        // To check whether a field is undefined, we can compare the value against "$$REMOVE"
        // Which returns true for missing values or undefined values
        doc! {
            "$ne": [field_name.into(), "$$REMOVE"]
        }
    } else {
        doc! {
            "$eq": [field_name.into(), "$$REMOVE"]
        }
    }
}

fn exclude_undefineds(field_name: impl Into<Bson>, invert: bool, filter: Document) -> Document {
    let is_set_filter = render_is_set(field_name, !invert);

    if invert {
        doc! { "$or": [filter, is_set_filter] }
    } else {
        doc! { "$and": [filter, is_set_filter] }
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

    pub fn render(&self) -> String {
        self.parts.join(".")
    }

    pub fn render_with(&self, target: String) -> String {
        if self.ignore_target {
            return format!("${}", self.render());
        }

        if self.parts.is_empty() {
            format!("${target}")
        } else {
            format!("${}.{}", self.render(), target)
        }
    }

    /// Sets whether the target should be rendered by the `render_with` method
    pub fn ignore_target(mut self, ignore_target: bool) -> Self {
        self.ignore_target = ignore_target;
        self
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

impl From<&ScalarFieldRef> for FilterPrefix {
    fn from(sf: &ScalarFieldRef) -> Self {
        Self {
            parts: vec![sf.db_name().to_owned()],
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
