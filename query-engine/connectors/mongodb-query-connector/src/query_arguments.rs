use crate::{
    cursor::{CursorBuilder, CursorData},
    filter::convert_filter,
    join::JoinStage,
    orderby::OrderByBuilder,
    BsonTransform, IntoBson,
};
use connector_interface::{AggregationSelection, Filter, QueryArguments};
use itertools::Itertools;
use mongodb::{
    bson::{doc, Bson, Document},
    options::{AggregateOptions, FindOptions},
    Collection, Cursor,
};
use prisma_models::{ModelProjection, ScalarFieldRef};

/// Translated query arguments ready to use in mongo find or aggregation queries.
#[derive(Debug, Default)]
pub(crate) struct MongoQueryArgs {
    /// Pre-join, "normal" filters.
    pub(crate) query: Option<Document>,

    /// Join stages.
    pub(crate) joins: Vec<JoinStage>,

    /// Filters that can only be applied after the joins
    /// or aggregations added the required data to execute them.
    pub(crate) join_filters: Vec<Document>,

    /// Aggregation stages.
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
    pub(crate) skip: Option<i64>,

    /// Take only a certain number of documents from the result.
    pub(crate) limit: Option<i64>,

    /// Projection document to scope down return fields.
    pub(crate) projection: Option<Document>,

    /// Switch to indicate the underlying aggregation is a `group_by` query.
    /// This is due to legacy drift in how `aggregate` and `group_by` work in
    /// the API and will hopefully be merged again in the future.
    pub(crate) is_group_by_query: bool,
}

impl MongoQueryArgs {
    pub(crate) fn new(args: QueryArguments) -> crate::Result<MongoQueryArgs> {
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

        Ok(MongoQueryArgs {
            query,
            join_filters: post_filters,
            joins,
            order_builder,
            cursor_builder,
            skip: skip(args.skip, args.ignore_skip),
            limit: take(args.take, args.ignore_take),
            ..Default::default()
        })
    }

    /// Turns the query args into a find operation on the collection.
    /// Depending on the arguments, either an aggregation pipeline or a plain query is build and run.
    pub(crate) async fn find_documents(mut self, coll: Collection) -> crate::Result<Cursor> {
        self.finalize()?;

        if self.joins.is_empty()
            && self.order_joins.is_empty()
            && self.aggregations.is_empty()
            && self.cursor_data.is_none()
        {
            self.execute_find_query(coll).await
        } else {
            self.execute_pipeline_query(coll).await
        }
    }

    /// Simplest form of find-documents query.
    /// Todo: Find out if always using aggregation pipeline is a good idea.
    async fn execute_find_query(self, coll: Collection) -> crate::Result<Cursor> {
        let find_options = FindOptions::builder()
            .projection(self.projection)
            .limit(self.limit)
            .skip(self.skip)
            .sort(self.order)
            .build();

        Ok(coll.find(self.query, find_options).await?)
    }

    async fn execute_pipeline_query(self, coll: Collection) -> crate::Result<Cursor> {
        if self.cursor_data.is_none() {
            self.execute_pipeline_internal(coll).await
        } else {
            self.execute_cursored_internal(coll).await
        }
    }

    /// Pipeline not requiring a cursor.
    async fn execute_pipeline_internal(self, coll: Collection) -> crate::Result<Cursor> {
        let opts = AggregateOptions::builder().allow_disk_use(true).build();
        let stages = self.into_pipeline_stages();

        dbg!(&stages);

        Ok(coll.aggregate(stages, opts).await?)
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
    /// The query has the form:
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
    async fn execute_cursored_internal(mut self, coll: Collection) -> crate::Result<Cursor> {
        let opts = AggregateOptions::builder().allow_disk_use(true).build();
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
                "from": coll.name(),
                "let": cursor_data.bindings,
                "pipeline": inner_stages,
                "as": "cursor_inner",
            }
        });

        outer_stages.push(doc! { "$unwind": "$cursor_inner" });
        outer_stages.push(doc! { "$replaceRoot": { "newRoot": "$cursor_inner" } });

        Ok(coll.aggregate(outer_stages, opts).await?)
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

        for selection in aggregations {
            match selection {
                AggregationSelection::Field(_) => (),
                AggregationSelection::Count { all, fields } => {
                    if *all {
                        grouping_stage.insert("count_all", doc! { "$sum": 1 });
                    }

                    let pairs = aggregation_pairs("count", fields);
                    grouping_stage.extend(pairs);
                }
                AggregationSelection::Average(fields) => {
                    let pairs = aggregation_pairs("avg", fields);
                    grouping_stage.extend(pairs);
                }
                AggregationSelection::Sum(fields) => {
                    let pairs = aggregation_pairs("sum", fields);
                    grouping_stage.extend(pairs);
                }
                AggregationSelection::Min(fields) => {
                    let pairs = aggregation_pairs("min", fields);
                    grouping_stage.extend(pairs);
                }
                AggregationSelection::Max(fields) => {
                    let pairs = aggregation_pairs("max", fields);
                    grouping_stage.extend(pairs);
                }
            }
        }

        self.aggregations.push(doc! { "$group": grouping_stage });
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

fn skip(skip: Option<i64>, ignore: bool) -> Option<i64> {
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
