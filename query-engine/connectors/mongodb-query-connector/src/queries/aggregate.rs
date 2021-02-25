use super::vacuum_cursor;
use crate::{query_arguments::MongoQueryArgs, value::value_from_bson};
use connector_interface::*;
use mongodb::{bson::Document, Database};
use prisma_models::prelude::*;

pub async fn aggregate(
    database: &Database,
    model: &ModelRef,
    query_arguments: QueryArguments,
    selections: Vec<AggregationSelection>,
    group_by: Vec<ScalarFieldRef>,
    having: Option<Filter>,
) -> crate::Result<Vec<AggregationRow>> {
    let coll = database.collection(&model.db_name());
    let mongo_args = MongoQueryArgs::new(query_arguments)?
        .with_groupings(group_by, &selections)
        .with_having(having)?;

    let cursor = mongo_args.find_documents(coll).await?;
    let docs = vacuum_cursor(cursor).await?;

    dbg!(&docs);

    to_aggregation_rows(docs, selections)
}

fn to_aggregation_rows(
    docs: Vec<Document>,
    selections: Vec<AggregationSelection>,
) -> crate::Result<Vec<AggregationRow>> {
    let mut rows = vec![];

    for mut doc in docs {
        let mut row = vec![];

        // The way we query guarantees that the _id key is always either Bson::Null or a document.
        // If a field is selected, it can never be Null, hence the unwraps are safe.
        let mut id_key_doc = doc.remove("_id").unwrap();

        for selection in selections.iter() {
            match selection {
                // All flat selection can only be in the _id part of the result doc.
                AggregationSelection::Field(f) => {
                    let field_val = id_key_doc.as_document_mut().unwrap().remove(f.db_name()).unwrap();
                    row.push(AggregationResult::Field(f.clone(), value_from_bson(field_val)?));
                }
                AggregationSelection::Count { all, fields } => {
                    if *all {
                        let field_val = value_from_bson(doc.remove("count_all").unwrap())?;
                        row.push(AggregationResult::Count(None, field_val));
                    } else {
                        for field in fields {
                            let field_val =
                                value_from_bson(doc.remove(&format!("count_{}", field.db_name())).unwrap())?;
                            row.push(AggregationResult::Count(Some(field.clone()), field_val));
                        }
                    }
                }
                AggregationSelection::Average(fields) => {
                    for field in fields {
                        let field_val = value_from_bson(doc.remove(&format!("avg_{}", field.db_name())).unwrap())?;
                        row.push(AggregationResult::Average(field.clone(), field_val));
                    }
                }
                AggregationSelection::Sum(fields) => {
                    for field in fields {
                        let field_val = value_from_bson(doc.remove(&format!("sum_{}", field.db_name())).unwrap())?;
                        row.push(AggregationResult::Sum(field.clone(), field_val));
                    }
                }
                AggregationSelection::Min(fields) => {
                    for field in fields {
                        let field_val = value_from_bson(doc.remove(&format!("min_{}", field.db_name())).unwrap())?;
                        row.push(AggregationResult::Min(field.clone(), field_val));
                    }
                }
                AggregationSelection::Max(fields) => {
                    for field in fields {
                        let field_val = value_from_bson(doc.remove(&format!("max_{}", field.db_name())).unwrap())?;
                        row.push(AggregationResult::Max(field.clone(), field_val));
                    }
                }
            };
        }

        rows.push(row);
    }

    Ok(rows)
}
