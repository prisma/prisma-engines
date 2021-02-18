use crate::{filter::convert_filter, BsonTransform, IntoBson};
use connector_interface::QueryArguments;
use mongodb::{bson::Document, options::FindOptions, Collection, Cursor};
use prisma_models::{ModelProjection, OrderBy, SortOrder};

/// Translated query arguments ready to use in mongo find or aggregation queries.
#[derive(Debug)]
pub(crate) struct MongoQueryArgs {
    pub(crate) query: Option<Document>,
    pub(crate) joins: Vec<Document>,
    pub(crate) projection: Document,
    pub(crate) order: Option<Document>,
    pub(crate) skip: Option<i64>,
    pub(crate) limit: Option<i64>,
}

impl MongoQueryArgs {
    /// Turns the query args into a find operation on the collection.
    /// Depending on the arguments, either an aggregation pipeline or a plain query is build and run.
    pub(crate) async fn execute_find(self, coll: Collection) -> crate::Result<Cursor> {
        let find_options = FindOptions::builder()
            .projection(self.projection)
            .limit(self.limit)
            .skip(self.skip)
            .sort(self.order)
            .build();

        Ok(coll.find(self.query, find_options).await?)
    }
}

/// Builds filter and find options for mongo based on selected fields and query arguments.
pub(crate) fn build_mongo_args(
    args: QueryArguments,
    selected_fields: ModelProjection,
) -> crate::Result<MongoQueryArgs> {
    let reverse_order = args.take.map(|t| t < 0).unwrap_or(false);
    let (order, mut joins) = build_order_by(args.order_by, reverse_order);
    let projection = selected_fields.into_bson()?.into_document()?;
    let query = match args.filter {
        Some(filter) => {
            let (filter, filter_joins) = convert_filter(filter)?;
            joins.extend(filter_joins);

            Some(filter)
        }
        None => None,
    };

    Ok(MongoQueryArgs {
        query,
        joins,
        projection,
        order,
        skip: skip(args.skip, args.ignore_skip),
        limit: take(args.take, args.ignore_take),
    })

    // let builder = FindOptions::builder()
    //     .projection()
    //     .sort()
    //     .skip()
    //     .limit(take(args.take, args.ignore_take));
}

fn build_order_by(orderings: Vec<OrderBy>, reverse: bool) -> (Option<Document>, Vec<Document>) {
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
