use connector_interface::{AggregationSelection, Filter, ScalarProjection};
use mongodb::bson::{doc, Bson, Document};
use prisma_models::ScalarFieldRef;
use std::collections::HashSet;

/// Represents a `$group` aggregation stage.
/// Groupings can be generated either from some `AggregationSelection` or a having `Filter`.
#[derive(Debug, Default)]
pub struct GroupByBuilder {
    /// A set of all aggregated fields.
    aggregations: HashSet<(ScalarFieldRef, AggregationType)>,
    /// Whether we need to group by count(*).
    count_all: bool,
}

/// A generic aggregation type that abstracts selections & filter aggregations.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum AggregationType {
    Count,
    Min,
    Max,
    Sum,
    Average,
}

impl GroupByBuilder {
    pub fn new() -> Self {
        Self { ..Default::default() }
    }

    pub fn render(&self, by_fields: Vec<ScalarFieldRef>) -> (Document, Option<Document>) {
        let grouping = if by_fields.is_empty() {
            Bson::Null // Null => group over the entire collection.
        } else {
            let mut group_doc = Document::new();

            for field in by_fields {
                group_doc.insert(field.db_name(), format!("${}", field.db_name()));
            }

            group_doc.into()
        };

        let mut group_stage = doc! { "_id": grouping };
        // Needed for field-count aggregations
        let mut project_stage = doc! {};

        if self.count_all {
            group_stage.insert("count_all", doc! { "$sum": 1 });
            project_stage.extend(projection_doc("count_all"));
        }

        for (sf, aggr_type) in &self.aggregations {
            match aggr_type {
                AggregationType::Count => {
                    // MongoDB requires a different construct for counting on fields.
                    // First, we push them into an array and then, in a separate project stage,
                    // we count the number of items in the array.
                    let push_pair = aggregation_pair("push", sf);
                    let (count_key, count_val) = count_field_pair(sf);

                    project_stage.insert(&count_key, doc! { "$sum": format!("${}", &count_key) });

                    group_stage.insert(push_pair.0, push_pair.1);
                    group_stage.insert(count_key, count_val);
                }
                AggregationType::Min => {
                    let (k, v) = aggregation_pair("min", sf);

                    project_stage.extend(projection_doc(&k));
                    group_stage.insert(k, v);
                }
                AggregationType::Max => {
                    let (k, v) = aggregation_pair("max", sf);

                    project_stage.extend(projection_doc(&k));
                    group_stage.insert(k, v);
                }
                AggregationType::Sum => {
                    let (k, v) = aggregation_pair("sum", sf);

                    project_stage.extend(projection_doc(&k));
                    group_stage.insert(k, v);
                }
                AggregationType::Average => {
                    let (k, v) = aggregation_pair("avg", sf);

                    project_stage.extend(projection_doc(&k));
                    group_stage.insert(k, v);
                }
            }
        }

        if self.requires_projection() {
            (group_stage, Some(project_stage))
        } else {
            (group_stage, None)
        }
    }

    /// Derives aggregated groupings from an `AggregationSelection`.
    pub fn with_selections(&mut self, selections: &[AggregationSelection]) {
        for selection in selections {
            match selection {
                AggregationSelection::Count { all, fields } => {
                    if *all {
                        self.count_all = true;
                    }

                    self.insert_groupings(fields, AggregationType::Count);
                }
                AggregationSelection::Average(fields) => {
                    self.insert_groupings(fields, AggregationType::Average);
                }
                AggregationSelection::Sum(fields) => {
                    self.insert_groupings(fields, AggregationType::Sum);
                }
                AggregationSelection::Min(fields) => {
                    self.insert_groupings(fields, AggregationType::Min);
                }
                AggregationSelection::Max(fields) => {
                    self.insert_groupings(fields, AggregationType::Max);
                }
                AggregationSelection::Field(_) => (),
            }
        }
    }

    /// Derives aggregated groupings from a having `Filter`.
    /// Required because the filter needs to match against a grouping,
    /// which is not present if no aggregation selection is made but an aggregation filter is used.
    pub fn with_having_filter(&mut self, having: &Filter) {
        let mut unfold_filters = |filters: &Vec<Filter>| {
            for filter in filters {
                self.with_having_filter(filter);
            }
        };

        match having {
            Filter::And(filters) => {
                unfold_filters(filters);
            }
            Filter::Or(filters) => {
                unfold_filters(filters);
            }
            Filter::Not(filters) => {
                unfold_filters(filters);
            }
            Filter::Aggregation(aggregation) => match aggregation {
                connector_interface::AggregationFilter::Count(filter) => {
                    self.insert_from_filter(filter.as_ref(), AggregationType::Count);
                }
                connector_interface::AggregationFilter::Average(filter) => {
                    self.insert_from_filter(filter.as_ref(), AggregationType::Average);
                }
                connector_interface::AggregationFilter::Sum(filter) => {
                    self.insert_from_filter(filter.as_ref(), AggregationType::Sum);
                }
                connector_interface::AggregationFilter::Min(filter) => {
                    self.insert_from_filter(filter.as_ref(), AggregationType::Min);
                }
                connector_interface::AggregationFilter::Max(filter) => {
                    self.insert_from_filter(filter.as_ref(), AggregationType::Max);
                }
            },
            _ => (),
        }
    }

    fn requires_projection(&self) -> bool {
        self.aggregations
            .iter()
            .any(|(_, aggr_type)| matches!(aggr_type, AggregationType::Count))
    }

    fn insert_from_filter(&mut self, filter: &Filter, aggregation_type: AggregationType) {
        let scalar_filter = filter.as_scalar().unwrap();
        let field = match &scalar_filter.projection {
            ScalarProjection::Single(sf) => sf,
            _ => unreachable!(),
        };

        self.insert_grouping(field, &aggregation_type);
    }

    fn insert_groupings(&mut self, fields: &[ScalarFieldRef], aggregation_type: AggregationType) {
        for field in fields {
            self.insert_grouping(field, &aggregation_type)
        }
    }

    fn insert_grouping(&mut self, field: &ScalarFieldRef, aggregation_type: &AggregationType) {
        self.aggregations.insert((field.clone(), aggregation_type.clone()));
    }
}

/// Produces pair like `("sum_fieldName", { "$sum": "$fieldName" })`.
/// Important: Only valid for non-count aggregations.
fn aggregation_pair(op: &str, field: &ScalarFieldRef) -> (String, Bson) {
    (
        format!("{}_{}", op, field.db_name()),
        doc! { format!("${}", op): format!("${}", field.db_name()) }.into(),
    )
}

/// Produces pair like `("count_fieldName", { "$sum": "$fieldName" })`.
/// Important: Only valid for field-level count aggregations.
fn count_field_pair(field: &ScalarFieldRef) -> (String, Bson) {
    (
        format!("count_{}", field.db_name()),
        doc! { "$push": { "$cond": { "if": format!("${}", field.db_name()), "then": 1, "else": 0 }}}.into(),
    )
}

/// Produces a document that projects a field.
/// Important: Only valid for non-count aggregations.
fn projection_doc(key: &str) -> Document {
    doc! { key: 1 }
}
