use crate::{constants::*, output_meta, query_builder::MongoReadQueryBuilder, value::value_from_bson};

use connector_interface::*;
use mongodb::{ClientSession, Database, bson::Document};
use query_structure::{AggregationSelection, Filter, QueryArguments, prelude::*};

pub async fn aggregate(
    database: &Database,
    session: &mut ClientSession,
    model: &Model,
    query_arguments: QueryArguments,
    selections: Vec<AggregationSelection>,
    group_by: Vec<ScalarFieldRef>,
    having: Option<Filter>,
) -> crate::Result<Vec<AggregationRow>> {
    let is_group_by = !group_by.is_empty();
    let coll = database.collection(model.db_name());

    let query = MongoReadQueryBuilder::from_args(query_arguments)?
        .with_groupings(group_by, &selections, having)?
        .build()?;

    let docs = query.execute(coll, session).await?;

    if is_group_by && docs.is_empty() {
        Ok(vec![])
    } else if docs.is_empty() {
        Ok(empty_aggregation(selections))
    } else {
        to_aggregation_rows(docs, selections)
    }
}

fn empty_aggregation(selections: Vec<AggregationSelection>) -> Vec<AggregationRow> {
    let mut row = vec![];

    for selection in selections.iter() {
        match selection {
            AggregationSelection::Field(f) => {
                row.push(AggregationResult::Field(f.clone(), PrismaValue::Null));
            }
            AggregationSelection::Count { all, fields } => {
                if all.is_some() {
                    row.push(AggregationResult::Count(None, PrismaValue::Int(0)));
                }

                for field in fields {
                    row.push(AggregationResult::Count(Some(field.clone()), PrismaValue::Int(0)));
                }
            }
            AggregationSelection::Average(fields) => {
                for field in fields {
                    row.push(AggregationResult::Average(field.clone(), PrismaValue::Null));
                }
            }
            AggregationSelection::Sum(fields) => {
                for field in fields {
                    row.push(AggregationResult::Sum(field.clone(), PrismaValue::Null));
                }
            }
            AggregationSelection::Min(fields) => {
                for field in fields {
                    row.push(AggregationResult::Min(field.clone(), PrismaValue::Null));
                }
            }
            AggregationSelection::Max(fields) => {
                for field in fields {
                    row.push(AggregationResult::Max(field.clone(), PrismaValue::Null));
                }
            }
        };
    }

    vec![row]
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
        let mut id_key_doc = doc.remove(group_by::UNDERSCORE_ID).unwrap();

        for selection in selections.iter() {
            let selection_meta = output_meta::from_aggregation_selection(selection);

            match selection {
                // All flat selection can only be in the _id part of the result doc.
                AggregationSelection::Field(f) => {
                    let field_val = id_key_doc.as_document_mut().unwrap().remove(f.db_name()).unwrap();
                    let meta = selection_meta.get(f.db_name()).unwrap();

                    row.push(AggregationResult::Field(f.clone(), value_from_bson(field_val, meta)?));
                }
                AggregationSelection::Count { all, fields } => {
                    if all.is_some() {
                        let meta = selection_meta.get("all").unwrap();
                        let field_val = value_from_bson(doc.remove("count_all").unwrap(), meta)?;

                        row.push(AggregationResult::Count(None, field_val));
                    }

                    for field in fields {
                        let meta = selection_meta.get(field.db_name()).unwrap();
                        let bson = doc.remove(format!("count_{}", field.db_name())).unwrap();
                        let field_val = value_from_bson(bson, meta)?;

                        row.push(AggregationResult::Count(Some(field.clone()), field_val));
                    }
                }
                AggregationSelection::Average(fields) => {
                    for field in fields {
                        let meta = selection_meta.get(field.db_name()).unwrap();
                        let bson = doc.remove(format!("avg_{}", field.db_name())).unwrap();
                        let field_val = value_from_bson(bson, meta)?;

                        row.push(AggregationResult::Average(field.clone(), field_val));
                    }
                }
                AggregationSelection::Sum(fields) => {
                    for field in fields {
                        let meta = selection_meta.get(field.db_name()).unwrap();
                        let bson = doc.remove(format!("sum_{}", field.db_name())).unwrap();
                        let field_val = value_from_bson(bson, meta)?;

                        row.push(AggregationResult::Sum(field.clone(), field_val));
                    }
                }
                AggregationSelection::Min(fields) => {
                    for field in fields {
                        let meta = selection_meta.get(field.db_name()).unwrap();
                        let bson = doc.remove(format!("min_{}", field.db_name())).unwrap();
                        let field_val = value_from_bson(bson, meta)?;

                        row.push(AggregationResult::Min(field.clone(), field_val));
                    }
                }
                AggregationSelection::Max(fields) => {
                    for field in fields {
                        let meta = selection_meta.get(field.db_name()).unwrap();
                        let bson = doc.remove(format!("max_{}", field.db_name())).unwrap();
                        let field_val = value_from_bson(bson, meta)?;

                        row.push(AggregationResult::Max(field.clone(), field_val));
                    }
                }
            };
        }

        rows.push(row);
    }

    Ok(rows)
}
