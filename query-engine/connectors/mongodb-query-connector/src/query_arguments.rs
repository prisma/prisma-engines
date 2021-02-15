use crate::{BsonTransform, IntoBson};
use connector_interface::QueryArguments;
use mongodb::{bson::Document, options::FindOptions};
use prisma_models::{ModelProjection, OrderBy, SortOrder};

/// Builds filter and find options for mongo based on selected fields and query arguments.
pub(crate) fn build_mongo_options(
    args: QueryArguments,
    selected_fields: &ModelProjection,
) -> crate::Result<(Option<Document>, Option<FindOptions>)> {
    let reverse_order = args.take.map(|t| t < 0).unwrap_or(false);
    let filter = match args.filter {
        Some(filter) => Some(filter.into_bson()?.into_document()?),
        None => None,
    };

    let builder = FindOptions::builder()
        .projection(selected_fields.clone().into_bson()?.into_document()?)
        .sort(build_order_by(args.order_by, reverse_order))
        .skip(skip(args.skip, args.ignore_skip))
        .limit(take(args.take, args.ignore_take));

    Ok((filter, Some(builder.build())))
}

fn build_order_by(orderings: Vec<OrderBy>, reverse: bool) -> Option<Document> {
    if orderings.is_empty() {
        return None;
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

    Some(order_doc)
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
