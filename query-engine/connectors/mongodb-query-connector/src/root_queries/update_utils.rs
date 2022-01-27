use super::*;
use crate::IntoBson;
use connector_interface::{DatasourceFieldName, NestedWrite, WriteExpression};
use itertools::Itertools;
use mongodb::bson::{doc, Document};
use prisma_models::PrismaValue;

pub fn render_update_docs(
    write_expr: WriteExpression,
    field: &Field,
    field_name: &str,
) -> crate::Result<Vec<Document>> {
    if let WriteExpression::NestedWrite(nested_write) = write_expr {
        let docs = unfold_nested_write(field, nested_write, &mut vec![field.db_name().to_owned()])
            .into_iter()
            // TODO: figure out why we can't flat_map here
            // TODO: the trait `FromIterator<Vec<mongodb::bson::Document>>` is not implemented for `std::result::Result<Vec<mongodb::bson::Document>, MongoError>`
            .map(|(write_expr, field, field_name)| render_update_docs(write_expr, field, &field_name))
            .collect::<crate::Result<Vec<_>>>()?;

        return Ok(docs.into_iter().flatten().collect_vec());
    };

    let dollar_field_name = format!("${}", field_name);

    let doc = match write_expr {
        WriteExpression::Add(rhs) if field.is_list() => match rhs {
            PrismaValue::List(vals) => {
                let vals = vals
                    .into_iter()
                    .map(|val| (field, val).into_bson())
                    .collect::<crate::Result<Vec<_>>>()?
                    .into_iter()
                    .map(|bson| {
                        // Strip the list from the BSON values. [Todo] This is unfortunately necessary right now due to how the
                        // conversion is set up with native types, we should clean that up at some point (move from traits to fns?).
                        if let Bson::Array(mut inner) = bson {
                            inner.pop().unwrap()
                        } else {
                            bson
                        }
                    })
                    .collect();

                let bson_array = Bson::Array(vals);

                doc! {
                    "$set": { field_name: {
                        "$ifNull": [
                            { "$concatArrays": [dollar_field_name, bson_array.clone()] },
                            bson_array
                        ]
                    } }
                }
            }
            val => {
                let bson_val = match (field, val).into_bson()? {
                    bson @ Bson::Array(_) => bson,
                    bson => Bson::Array(vec![bson]),
                };

                doc! {
                    "$set": {
                        field_name: {
                            "$ifNull": [
                                { "$concatArrays": [dollar_field_name, bson_val.clone()] },
                                bson_val
                            ]
                        }
                    }
                }
            }
        },
        // We use $literal to enable the set of empty object, which is otherwise considered a syntax error
        WriteExpression::Value(rhs) => doc! {
            "$set": { field_name: { "$literal": (field, rhs).into_bson()? } }
        },
        WriteExpression::Add(rhs) => doc! {
            "$set": { field_name: { "$add": [dollar_field_name, (field, rhs).into_bson()?] } }
        },
        WriteExpression::Substract(rhs) => doc! {
            "$set": { field_name: { "$subtract": [dollar_field_name, (field, rhs).into_bson()?] } }
        },
        WriteExpression::Multiply(rhs) => doc! {
            "$set": { field_name: { "$multiply": [dollar_field_name, (field, rhs).into_bson()?] } }
        },
        WriteExpression::Divide(rhs) => doc! {
            "$set": { field_name: { "$divide": [dollar_field_name, (field, rhs).into_bson()?] } }
        },
        WriteExpression::NestedWrite(_) => unreachable!(),
        WriteExpression::Field(_) => unimplemented!(),
    };

    Ok(vec![doc])
}

fn unfold_nested_write<'a>(
    field: &'a Field,
    nested_write: NestedWrite,
    path: &mut Vec<String>,
) -> Vec<(WriteExpression, &'a Field, String)> {
    let mut nested_writes: Vec<(WriteExpression, &'a Field, String)> = vec![];

    for (DatasourceFieldName(db_name), write) in nested_write.writes {
        let nested_field = field
            .as_composite()
            .unwrap()
            .typ
            .find_field_by_db_name(&db_name)
            .unwrap();

        match write {
            WriteExpression::NestedWrite(nested_write) => {
                let mut path = path.clone();
                path.push(db_name);

                nested_writes.extend(unfold_nested_write(nested_field, nested_write, &mut path));
            }
            _ => {
                path.push(db_name);
                nested_writes.push((write, nested_field, path.join(".").to_owned()));
            }
        }
    }

    nested_writes
}
