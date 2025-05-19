use super::group_by_builder::*;

use crate::{
    constants::*,
    cursor::{CursorBuilder, CursorData},
    filter::{FilterPrefix, MongoFilterVisitor},
    join::JoinStage,
    orderby::OrderByBuilder,
    query_strings::Aggregate,
    root_queries::observing,
    vacuum_cursor, BsonTransform, IntoBson,
};
use bson::{doc, Document};
use itertools::Itertools;
use mongodb::{options::AggregateOptions, ClientSession, Collection};
use query_structure::{
    AggregationSelection, FieldSelection, Filter, Model, QueryArguments, ScalarFieldRef, Take, VirtualSelection,
};
use std::convert::TryFrom;
use std::future::IntoFuture;

// Mongo Driver broke usage of the simple API, can't be used by us anymore.
// As such the read query will always be based on aggregation pipeline
// such pipeline will have different stages. See
// https://www.mongodb.com/docs/manual/core/aggregation-pipeline/
pub struct ReadQuery {
    pub(crate) stages: Vec<Document>,
}

impl ReadQuery {
    pub async fn execute(
        self,
        on_collection: Collection<Document>,
        with_session: &mut ClientSession,
    ) -> crate::Result<Vec<Document>> {
        let opts = AggregateOptions::builder().allow_disk_use(true).build();
        let query_string_builder = Aggregate::new(&self.stages, on_collection.name());
        let cursor = observing(&query_string_builder, || {
            on_collection
                .aggregate(self.stages.clone())
                .with_options(opts)
                .session(&mut *with_session)
                .into_future()
        })
        .await?;

        vacuum_cursor(cursor, with_session).await
    }
}

/// Translated query arguments ready to use in mongo find or aggregation queries.
#[derive(Debug)]
pub(crate) struct MongoReadQueryBuilder {
    pub(crate) model: Model,

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

    /// Finalized ordering aggregation computed from the joins
    pub(crate) order_aggregate_projections: Vec<Document>,

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
    pub fn new(model: Model) -> Self {
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
            order_aggregate_projections: vec![],
            cursor_builder: None,
            cursor_data: None,
            skip: None,
            limit: None,
            projection: None,
            is_group_by_query: false,
        }
    }

    pub(crate) fn from_args(args: QueryArguments) -> crate::Result<MongoReadQueryBuilder> {
        let reverse_order = args.take.is_reversed();
        let order_by = args.order_by;

        let order_builder = Some(OrderByBuilder::new(order_by.clone(), reverse_order));
        let cursor_builder = args.cursor.map(|c| CursorBuilder::new(c, order_by, reverse_order));

        let mut post_filters = vec![];
        let mut joins = vec![];

        let query = match args.filter {
            Some(filter) => {
                // If a filter comes with joins, it needs to be run _after_ the initial filter query / $matches.
                let (filter, filter_joins) = MongoFilterVisitor::new(FilterPrefix::default(), false)
                    .visit(filter)?
                    .render();
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
            order_aggregate_projections: vec![],
            cursor_data: None,
            projection: None,
            is_group_by_query: false,
        })
    }

    /// Finalizes the builder and builds a `MongoQuery`.
    pub(crate) fn build(mut self) -> crate::Result<ReadQuery> {
        self.finalize()?;
        Ok(self.build_pipeline_query())
    }

    /// Aggregation-pipeline based query. A distinction must be made between cursored and uncursored queries,
    /// as they require different stage shapes (see individual fns for details).
    fn build_pipeline_query(self) -> ReadQuery {
        let stages = if self.cursor_data.is_none() {
            self.into_pipeline_stages()
        } else {
            self.cursored_pipeline_stages()
        };

        ReadQuery { stages }
    }

    fn into_pipeline_stages(self) -> Vec<Document> {
        let mut stages = vec![];

        // Initial $matches
        if let Some(query) = self.query {
            stages.push(doc! { "$match": { "$expr": query } })
        };

        // Joins ($lookup)
        let joins = self.joins.into_iter().chain(self.order_joins);

        let mut unwinds: Vec<Document> = vec![];

        for join_stage in joins {
            let (join, unwind) = join_stage.build();

            if let Some(u) = unwind {
                unwinds.push(u);
            }

            stages.push(join);
        }

        // Order by aggregate computed from joins ($addFields)
        stages.extend(self.order_aggregate_projections);

        // Post-join $matches
        stages.extend(
            self.join_filters
                .into_iter()
                .map(|filter| doc! { "$match": { "$expr": filter } }),
        );

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
                    .map(|filter| doc! { "$match": { "$expr": filter } }),
            );
        }

        // Join's $unwind placed before sorting
        // because Mongo does not support sorting multiple arrays
        // https://jira.mongodb.org/browse/SERVER-32859
        stages.extend(unwinds);

        // $sort
        if let Some(order) = self.order {
            stages.push(doc! { "$sort": order })
        };

        // $skip
        if let Some(skip) = self.skip {
            stages.push(doc! { "$skip": i64::try_from(skip).unwrap() });
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
                    .map(|filter| doc! { "$match": { "$expr": filter } }),
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
            .map(|nested_stage| {
                let (join, _) = nested_stage.build();

                join
            })
            .collect_vec();

        // Outer query to pin the cursor document.
        let mut outer_stages = vec![];

        // First match the cursor, then add required ordering joins.
        outer_stages.push(doc! { "$match": { "$expr": cursor_data.cursor_filter } });
        outer_stages.extend(order_join_stages);

        outer_stages.extend(self.order_aggregate_projections.clone());

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

    /// Adds a final projection onto the fields specified by the `FieldSelection`.
    pub fn with_model_projection(mut self, selected_fields: FieldSelection) -> crate::Result<Self> {
        let projection = selected_fields.into_bson()?.into_document()?;
        self.projection = Some(projection);

        Ok(self)
    }

    /// Adds the necessary joins and the associated selections to the projection
    pub fn with_virtual_fields<'a>(
        mut self,
        virtual_selections: impl Iterator<Item = &'a VirtualSelection>,
    ) -> crate::Result<Self> {
        for aggr in virtual_selections {
            let join = match aggr {
                VirtualSelection::RelationCount(rf, filter) => {
                    let filter = filter
                        .as_ref()
                        .map(|f| MongoFilterVisitor::new(FilterPrefix::default(), false).visit(f.clone()))
                        .transpose()?;

                    JoinStage {
                        source: rf.clone(),
                        alias: Some(aggr.db_alias()),
                        nested: vec![],
                        filter,
                    }
                }
            };

            let projection = doc! {
              aggr.db_alias(): { "$size": format!("${}", aggr.db_alias()) }
            };

            self.joins.push(join);
            self.projection = self.projection.map_or(Some(projection.clone()), |mut p| {
                p.extend(projection);
                Some(p)
            });
        }

        Ok(self)
    }

    /// Adds group-by fields with their aggregations to this query.
    pub fn with_groupings(
        mut self,
        by_fields: Vec<ScalarFieldRef>,
        selections: &[AggregationSelection],
        having: Option<Filter>,
    ) -> crate::Result<Self> {
        if !by_fields.is_empty() {
            self.is_group_by_query = true;
        }

        let mut group_by = GroupByBuilder::new();
        group_by.with_selections(selections);

        if let Some(having) = having {
            group_by.with_having_filter(&having);

            // Having filters can only appear in group by queries.
            // All group by fields go into the UNDERSCORE_ID key of the result document.
            // As it is the only place where the flat scalars are contained for the group,
            // we need to refer to that object.
            let prefix = FilterPrefix::from(group_by::UNDERSCORE_ID);
            let (filter_doc, _) = MongoFilterVisitor::new(prefix, false).visit(having)?.render();

            self.aggregation_filters.push(filter_doc);
        }

        let (grouping_stage, project_stage) = group_by.render(by_fields);

        self.aggregations.push(doc! { "$group": grouping_stage });

        if let Some(project_stage) = project_stage {
            self.aggregations.push(doc! { "$project": project_stage });
        }

        Ok(self)
    }

    /// Runs last transformations on `self` to execute steps dependent on base args.
    fn finalize(&mut self) -> crate::Result<()> {
        // Cursor building depends on the ordering, so it must come first.
        if let Some(order_builder) = self.order_builder.take() {
            let (order, order_aggregate_projections, joins) = order_builder.build(self.is_group_by_query);

            self.order_joins.extend(joins);
            self.order = order;
            self.order_aggregate_projections = order_aggregate_projections;
        }

        if let Some(cursor_builder) = self.cursor_builder.take() {
            let cursor_data = cursor_builder.build()?;

            self.cursor_data = Some(cursor_data);
        }

        Ok(())
    }
}

fn skip(skip: Option<u64>, ignore: bool) -> Option<u64> {
    if ignore {
        None
    } else {
        skip
    }
}

fn take(take: Take, ignore: bool) -> Option<i64> {
    if ignore {
        None
    } else {
        match take {
            Take::All => None,
            Take::One => Some(1),
            Take::Some(n) => Some(n.abs()),
        }
    }
}
