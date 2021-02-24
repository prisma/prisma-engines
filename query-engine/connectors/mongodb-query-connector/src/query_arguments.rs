use crate::{filter::convert_filter, join::JoinStage, BsonTransform, IntoBson};
use connector_interface::QueryArguments;
use mongodb::{
    bson::{doc, Document},
    options::{AggregateOptions, FindOptions},
    Collection, Cursor,
};
use prisma_models::{ModelProjection, OrderBy, SortOrder};

/// Translated query arguments ready to use in mongo find or aggregation queries.
#[derive(Debug, Default)]
pub(crate) struct MongoQueryArgs {
    /// Pre-join, "normal" filters.
    pub(crate) query: Option<Document>,

    /// Join stages.
    pub(crate) joins: Vec<JoinStage>,

    /// Aggregation stages.
    pub(crate) aggregations: Vec<Document>,

    /// Filters that can only be applied after the joins
    /// or aggregations added the required data to execute them.
    pub(crate) post_filters: Vec<Document>,

    /// Order by expression document.
    pub(crate) order: Option<Document>,

    /// Skip a number of documents at the start of the result.
    pub(crate) skip: Option<i64>,

    /// Take only a certain number of documents from the result.
    pub(crate) limit: Option<i64>,

    /// Projection document to scope down return fields.
    pub(crate) projection: Option<Document>,
}

impl MongoQueryArgs {
    /// Turns the query args into a find operation on the collection.
    /// Depending on the arguments, either an aggregation pipeline or a plain query is build and run.
    pub(crate) async fn find_documents(self, coll: Collection) -> crate::Result<Cursor> {
        if self.joins.is_empty() && self.aggregations.is_empty() {
            self.execute_find_query(coll).await
        } else {
            self.execute_pipeline_query(coll).await
        }
    }

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
        let opts = AggregateOptions::builder().allow_disk_use(true).build();
        let mut stages = vec![];

        // Initial $matches
        if let Some(query) = self.query {
            stages.push(doc! { "$match": query })
        };

        // Joins ($lookup)
        stages.extend(self.joins.into_iter().map(|stage| doc! { "$lookup": stage.build() }));

        // Post-join $matches
        stages.extend(self.post_filters.into_iter().map(|filter| doc! { "$match": filter }));

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

        dbg!(&stages);

        Ok(coll.aggregate(stages, opts).await?)
    }

    /// Builds filter and find options for mongo based on selected fields and query arguments.
    pub(crate) fn new(args: QueryArguments) -> crate::Result<MongoQueryArgs> {
        let reverse_order = args.take.map(|t| t < 0).unwrap_or(false);
        let (order, mut joins) = build_order_by(args.order_by, reverse_order);

        let mut post_filters = vec![];

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
            post_filters,
            joins,
            order,
            skip: skip(args.skip, args.ignore_skip),
            limit: take(args.take, args.ignore_take),
            ..Default::default()
        })
    }

    /// Adds a final projection onto the fields specified by the `ModelProjection`.
    pub fn with_model_projection(mut self, selected_fields: ModelProjection) -> crate::Result<Self> {
        let projection = selected_fields.into_bson()?.into_document()?;
        self.projection = Some(projection);

        Ok(self)
    }
}

fn build_order_by(orderings: Vec<OrderBy>, reverse: bool) -> (Option<Document>, Vec<JoinStage>) {
    if orderings.is_empty() {
        return (None, vec![]);
    }

    let mut order_doc = Document::new();

    for order_by in orderings {
        // Mongo: -1 -> DESC, 1 -> ASC
        match (order_by.sort_order, reverse) {
            (SortOrder::Ascending, true) => order_doc.insert(order_by.field.db_name(), -1),
            (SortOrder::Descending, true) => order_doc.insert(order_by.field.db_name(), 1),
            (SortOrder::Ascending, false) => order_doc.insert(order_by.field.db_name(), 1),
            (SortOrder::Descending, false) => order_doc.insert(order_by.field.db_name(), -1),
        };
    }

    // todo joins
    (Some(order_doc), vec![])
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
