use super::logger::*;
use crate::{
    cursor::{CursorBuilder, CursorData},
    filter::convert_filter,
    join::JoinStage,
    orderby::OrderByBuilder,
    vacuum_cursor, BsonTransform, IntoBson,
};
use connector_interface::{AggregationSelection, Filter, QueryArguments};
use itertools::Itertools;
use mongodb::{
    bson::{doc, Bson, Document},
    options::{AggregateOptions, FindOptions},
    ClientSession, Collection,
};
use prisma_models::{ModelProjection, ModelRef, ScalarFieldRef};

/// Ergonomics wrapper for query execution and logging.
/// Todo: Add all other queries gradually.
#[allow(dead_code)]
pub enum MongoReadQuery {
    Find(FindQuery),
    Pipeline(PipelineQuery),
}

impl MongoReadQuery {
    pub async fn execute(
        self,
        on_collection: Collection,
        with_session: &mut ClientSession,
    ) -> crate::Result<Vec<Document>> {
        log_query(on_collection.name(), &self);
        match self {
            MongoReadQuery::Find(q) => q.execute(on_collection, with_session).await,
            MongoReadQuery::Pipeline(q) => q.execute(on_collection, with_session).await,
        }
    }
}

pub struct PipelineQuery {
    pub(crate) stages: Vec<Document>,
}

impl PipelineQuery {
    pub async fn execute(
        self,
        on_collection: Collection,
        with_session: &mut ClientSession,
    ) -> crate::Result<Vec<Document>> {
        let opts = AggregateOptions::builder().allow_disk_use(true).build();
        let cursor = on_collection
            .aggregate_with_session(self.stages, opts, with_session)
            .await?;

        Ok(vacuum_cursor(cursor, with_session).await?)
    }
}

pub struct FindQuery {
    pub(crate) filter: Option<Document>,
    pub(crate) options: FindOptions,
}

impl FindQuery {
    pub async fn execute(
        self,
        on_collection: Collection,
        with_session: &mut ClientSession,
    ) -> crate::Result<Vec<Document>> {
        let cursor = on_collection
            .find_with_session(self.filter, self.options, with_session)
            .await?;

        Ok(vacuum_cursor(cursor, with_session).await?)
    }
}

/// Translated query arguments ready to use in mongo find or aggregation queries.
#[derive(Debug)]
pub(crate) struct MongoReadQueryBuilder {
    pub(crate) model: ModelRef,

    /// Pre-join, "normal" filters.
    pub(crate) query: Option<Document>,

    /// Join stages.
    pub(crate) joins: Vec<JoinStage>,

    /// Filters that can only be applied after the joins
    /// or aggregations added the required data to execute them.
    pub(crate) join_filters: Vec<Document>,

    /// Aggregation-related stages.
    pub(crate) aggregations: Vec<Document>,

    /// Filters that can only be applied after the aggregations
    /// transformed the documents.
    pub(crate) aggregation_filters: Vec<Document>,

    /// Order by builder for deferred processing.
    order_builder: Option<OrderByBuilder>,

    /// Finalized ordering: Order document.
    pub(crate) order: Option<Document>,

    /// Finalized ordering: Necessary Joins
    /// Kept separate as cursor building needs to consider them seperately.
    pub(crate) order_joins: Vec<JoinStage>,

    /// Cursor builder for deferred processing.
    cursor_builder: Option<CursorBuilder>,

    /// Struct containing data required to build cursor queries.
    pub(crate) cursor_data: Option<CursorData>,

    /// Skip a number of documents at the start of the result.
    pub(crate) skip: Option<u64>,

    /// Take only a certain number of documents from the result.
    pub(crate) limit: Option<i64>,

    /// Projection document to scope down return fields.
    pub(crate) projection: Option<Document>,

    /// Switch to indicate the underlying aggregation is a `group_by` query.
    /// This is due to legacy drift in how `aggregate` and `group_by` work in
    /// the API and will hopefully be merged again in the future.
    pub(crate) is_group_by_query: bool,
}

impl MongoReadQueryBuilder {
    pub fn _new(model: ModelRef) -> Self {
        Self {
            model,
            query: None,
            joins: vec![],
            join_filters: vec![],
            aggregations: vec![],
            aggregation_filters: vec![],
            order_builder: None,
            order: None,
            order_joins: vec![],
            cursor_builder: None,
            cursor_data: None,
            skip: None,
            limit: None,
            projection: None,
            is_group_by_query: false,
        }
    }

    pub(crate) fn from_args(args: QueryArguments) -> crate::Result<MongoReadQueryBuilder> {
        let reverse_order = args.take.map(|t| t < 0).unwrap_or(false);
        let order_by = args.order_by;

        let order_builder = Some(OrderByBuilder::new(order_by.clone(), reverse_order));
        let cursor_builder = args.cursor.map(|c| CursorBuilder::new(c, order_by, reverse_order));

        let mut post_filters = vec![];
        let mut joins = vec![];

        let query = match args.filter {
            Some(filter) => {
                // If a filter comes with joins, it needs to be run _after_ the initial filter query / $matches.
                let (filter, filter_joins) = convert_filter(filter, false)?.render();
                if !filter_joins.is_empty() {
                    joins.extend(filter_joins);
                    post_filters.push(filter);

                    None
                } else {
                    Some(filter)
                }
            }
            None => None,
        };

        Ok(MongoReadQueryBuilder {
            model: args.model,
            query,
            join_filters: post_filters,
            joins,
            order_builder,
            cursor_builder,
            skip: skip(args.skip.map(|i| i as u64), args.ignore_skip),
            limit: take(args.take, args.ignore_take),
            aggregations: vec![],
            aggregation_filters: vec![],
            order: None,
            order_joins: vec![],
            cursor_data: None,
            projection: None,
            is_group_by_query: false,
        })
    }

    /// Finalizes the builder and builds a `MongoQuery`.
    pub(crate) fn build(mut self) -> crate::Result<MongoReadQuery> {
        self.finalize()?;

        // Depending on the builder contents, either an
        // aggregation pipeline or a plain query is build.
        // if self.joins.is_empty()
        //     && self.order_joins.is_empty()
        //     && self.aggregations.is_empty()
        //     && self.cursor_data.is_none()
        // {
        //     Ok(self.build_find_query())
        // } else {
        // }

        Ok(self.build_pipeline_query())
    }

    /// Note: Mongo Driver broke usage of the simple API, can't be used by us anymore. Always doing aggr. pipeline for now.
    /// Simplest form of find-documents query: `coll.find(filter, opts)`.
    #[allow(dead_code)]
    fn build_find_query(self) -> MongoReadQuery {
        // let options = FindOptions::builder()
        //     .projection(self.projection)
        //     .limit(self.limit)
        //     .skip(self.skip)
        //     .sort(self.order)
        //     .build();

        // MongoReadQuery::Find(FindQuery {
        //     filter: self.query,
        //     options,
        // })

        unreachable!()
    }

    /// Aggregation-pipeline based query. A distinction must be made between cursored and uncursored queries,
    /// as they require different stage shapes (see individual fns for details).
    fn build_pipeline_query(self) -> MongoReadQuery {
        let stages = if self.cursor_data.is_none() {
            self.flat_pipeline_stages()
        } else {
            self.cursored_pipeline_stages()
        };

        MongoReadQuery::Pipeline(PipelineQuery { stages })
    }

    /// Pipeline not requiring a cursor. Flat `coll.aggregate(stages, opts)` query.
    fn flat_pipeline_stages(self) -> Vec<Document> {
        self.into_pipeline_stages()
    }

    fn into_pipeline_stages(self) -> Vec<Document> {
        let mut stages = vec![];

        // Initial $matches
        if let Some(query) = self.query {
            stages.push(doc! { "$match": query })
        };

        // Joins ($lookup)
        let joins = self.joins.into_iter().chain(self.order_joins);

        stages.extend(joins.flat_map(|nested_stage| {
            let (join, unwind) = nested_stage.build();

            match unwind {
                Some(unwind) => vec![join, unwind],
                None => vec![join],
            }
        }));

        // Post-join $matches
        stages.extend(self.join_filters.into_iter().map(|filter| doc! { "$match": filter }));

        // If the query is a group by, then skip, take, sort all apply to the _groups_, not the documents
        // before. If it is a plain aggregation, then the aggregate stages need to be _after_ these, because
        // they apply to the documents to be aggregated, not the aggregations (legacy meh).
        if self.is_group_by_query {
            // Aggregates
            stages.extend(self.aggregations.clone());

            // Aggregation filters
            stages.extend(
                self.aggregation_filters
                    .clone()
                    .into_iter()
                    .map(|filter| doc! { "$match": filter }),
            );
        }

        // $sort
        if let Some(order) = self.order {
            stages.push(doc! { "$sort": order })
        };

        // $skip
        if let Some(skip) = self.skip {
            stages.push(doc! { "$skip": skip });
        };

        // $limit
        if let Some(limit) = self.limit {
            stages.push(doc! { "$limit": limit });
        };

        // $project
        if let Some(projection) = self.projection {
            stages.push(doc! { "$project": projection });
        };

        if !self.is_group_by_query {
            // Aggregates
            stages.extend(self.aggregations);

            // Aggregation filters
            stages.extend(
                self.aggregation_filters
                    .into_iter()
                    .map(|filter| doc! { "$match": filter }),
            );
        }

        stages
    }

    /// Pipeline query with a cursor. Requires special building to form a query that first
    /// pins a cursor and then builds cursor conditions based on that cursor document
    /// and the orderings that the query defined.
    /// The stages have the form:
    /// ```text
    /// testModel.aggregate([
    ///     { $match: { <filter finding exactly one document (cursor doc)> }},
    ///     { $lookup: { <if present, join that are required for orderBy relations> }}
    ///     { ... more joins if necessary ... }
    ///     {
    ///         $lookup: <"self join" testModel and execute non-cursor query with cursor filter here.>
    ///     }
    /// ])
    /// ```
    /// Expressed in words, this query first makes the cursor document (that defines all values
    /// to make cursor comparators work) available for the inner pipeline to build the filters.
    /// The inner pipeline is basically what an non-cursor query would look like with added cursor
    /// conditions. The inner join stage is refered to as a self-join here because it joins the cursor document
    /// to it's collection again to pull in all documents for filtering, but technically it doesn't
    /// actually join anything.
    ///
    /// Todo concrete example
    fn cursored_pipeline_stages(mut self) -> Vec<Document> {
        let coll_name = self.model.db_name().to_owned();
        let cursor_data = self.cursor_data.take().unwrap();

        // For now we assume that simply putting the cursor condition into the join conditions is enough
        // to let them run in the correct place.
        self.join_filters.push(cursor_data.cursor_condition);

        let order_join_stages = self
            .order_joins
            .clone()
            .into_iter()
            .flat_map(|nested_stage| {
                let (join, unwind) = nested_stage.build();

                match unwind {
                    Some(unwind) => vec![join, unwind],
                    None => vec![join],
                }
            })
            .collect_vec();

        // Outer query to pin the cursor document.
        let mut outer_stages = vec![];

        // First match the cursor, then add required ordering joins.
        outer_stages.push(doc! { "$match": cursor_data.cursor_filter });
        outer_stages.extend(order_join_stages);

        // Self-"join" collection
        let inner_stages = self.into_pipeline_stages();

        outer_stages.push(doc! {
            "$lookup": {
                "from": coll_name,
                "let": cursor_data.bindings,
                "pipeline": inner_stages,
                "as": "cursor_inner",
            }
        });

        outer_stages.push(doc! { "$unwind": "$cursor_inner" });
        outer_stages.push(doc! { "$replaceRoot": { "newRoot": "$cursor_inner" } });

        outer_stages
    }

    /// Adds a final projection onto the fields specified by the `ModelProjection`.
    pub fn with_model_projection(mut self, selected_fields: ModelProjection) -> crate::Result<Self> {
        let projection = selected_fields.into_bson()?.into_document()?;
        self.projection = Some(projection);

        Ok(self)
    }

    /// Adds group-by fields with their aggregations to this query.
    pub fn with_groupings(mut self, by_fields: Vec<ScalarFieldRef>, aggregations: &[AggregationSelection]) -> Self {
        let grouping = if by_fields.is_empty() {
            Bson::Null // Null => group over the entire collection.
        } else {
            let mut group_doc = Document::new();
            self.is_group_by_query = true;

            for field in by_fields {
                group_doc.insert(field.db_name(), format!("${}", field.db_name()));
            }

            group_doc.into()
        };

        let mut grouping_stage = doc! { "_id": grouping };

        // Needed for field-count aggregations
        let mut project_stage = doc! {};

        let requires_projection = aggregations
            .iter()
            .any(|a| matches!(a, AggregationSelection::Count { all: _, fields } if !fields.is_empty()));

        for selection in aggregations {
            match selection {
                AggregationSelection::Field(_) => (),
                AggregationSelection::Count { all, fields } => {
                    if *all {
                        grouping_stage.insert("count_all", doc! { "$sum": 1 });
                        project_stage.insert("count_all", Bson::Int64(1));
                    }

                    // MongoDB requires a different construct for counting on fields.
                    // First, we push them into an array and then, in a separate project stage,
                    // we count the number of items in the array.
                    let pairs = aggregation_pairs("push", fields);
                    grouping_stage.extend(pairs);

                    let grouping_pairs = count_field_pairs(fields);
                    let projection_pairs = grouping_pairs
                        .iter()
                        .map(|(a, _)| (a.clone(), doc! { "$sum": format!("${}", a) }.into()))
                        .collect_vec();

                    grouping_stage.extend(grouping_pairs);
                    project_stage.extend(projection_pairs);
                }
                AggregationSelection::Average(fields) => {
                    let grouping_pairs = aggregation_pairs("avg", fields);
                    let projection_pairs = grouping_pairs
                        .iter()
                        .map(|(a, _)| (a.clone(), Bson::Int64(1)))
                        .collect_vec();

                    grouping_stage.extend(grouping_pairs);
                    project_stage.extend(projection_pairs);
                }
                AggregationSelection::Sum(fields) => {
                    let grouping_pairs = aggregation_pairs("sum", fields);
                    let projection_pairs = grouping_pairs
                        .iter()
                        .map(|(a, _)| (a.clone(), Bson::Int64(1)))
                        .collect_vec();

                    grouping_stage.extend(grouping_pairs);
                    project_stage.extend(projection_pairs);
                }
                AggregationSelection::Min(fields) => {
                    let grouping_pairs = aggregation_pairs("min", fields);
                    let projection_pairs = grouping_pairs
                        .iter()
                        .map(|(a, _)| (a.clone(), Bson::Int64(1)))
                        .collect_vec();

                    grouping_stage.extend(grouping_pairs);
                    project_stage.extend(projection_pairs);
                }
                AggregationSelection::Max(fields) => {
                    let grouping_pairs = aggregation_pairs("max", fields);
                    let projection_pairs = grouping_pairs
                        .iter()
                        .map(|(a, _)| (a.clone(), Bson::Int64(1)))
                        .collect_vec();

                    grouping_stage.extend(grouping_pairs);
                    project_stage.extend(projection_pairs);
                }
            }
        }

        self.aggregations.push(doc! { "$group": grouping_stage });

        if requires_projection {
            self.aggregations.push(doc! { "$project": project_stage });
        }

        self
    }

    /// Adds aggregation filters based on a having scalar filter.
    pub fn with_having(mut self, having: Option<Filter>) -> crate::Result<Self> {
        if let Some(filter) = having {
            let (filter_doc, _) = convert_filter(filter, false)?.render();
            self.aggregation_filters.push(filter_doc);
        }

        Ok(self)
    }

    /// Runs last transformations on `self` to execute steps dependent on base args.
    fn finalize(&mut self) -> crate::Result<()> {
        // Cursor building depends on the ordering, so it must come first.
        if let Some(order_builder) = self.order_builder.take() {
            let (order, joins) = order_builder.build(self.is_group_by_query);

            self.order_joins.extend(joins);
            self.order = order;
        }

        if let Some(cursor_builder) = self.cursor_builder.take() {
            let cursor_data = cursor_builder.build()?;

            self.cursor_data = Some(cursor_data);
        }

        Ok(())
    }
}

/// Utilities below ///

/// Produces pairs like `("count_fieldName", { "$sum": "$fieldName" })`.
/// Important: Only valid for field-level count aggregations.
fn count_field_pairs(fields: &[ScalarFieldRef]) -> Vec<(String, Bson)> {
    fields
        .iter()
        .map(|field| {
            (
                format!("count_{}", field.db_name()),
                doc! { "$push": { "$cond": { "if": format!("${}", field.db_name()), "then": 1, "else": 0 }}}.into(),
            )
        })
        .collect()
}

/// Produces pairs like `("sum_fieldName", { "$sum": "$fieldName" })`.
/// Important: Only valid for non-count aggregations.
fn aggregation_pairs(op: &str, fields: &[ScalarFieldRef]) -> Vec<(String, Bson)> {
    fields
        .iter()
        .map(|field| {
            (
                format!("{}_{}", op, field.db_name()),
                doc! { format!("${}", op): format!("${}", field.db_name()) }.into(),
            )
        })
        .collect()
}

fn skip(skip: Option<u64>, ignore: bool) -> Option<u64> {
    if ignore {
        None
    } else {
        skip
    }
}

fn take(take: Option<i64>, ignore: bool) -> Option<i64> {
    if ignore {
        None
    } else {
        take.map(|t| if t < 0 { -t } else { t })
    }
}
