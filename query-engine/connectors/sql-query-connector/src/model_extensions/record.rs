use super::ScalarFieldExt;
use crate::SqlError;
use prisma_models::{DomainError, ModelProjection, RecordProjection};
use quaint::{connector::ResultSet, Value};
use std::convert::TryInto;

pub fn try_convert(model_projection: &ModelProjection, result_set: ResultSet) -> crate::Result<RecordProjection> {
    let columns: Vec<String> = result_set.columns().iter().map(|c| c.to_string()).collect();
    let mut record_projection = RecordProjection::default();

    if let Some(row) = result_set.into_iter().next() {
        for (i, val) in row.into_iter().enumerate() {
            match model_projection.map_db_name(columns[i].as_str()) {
                Some(field) => {
                    record_projection.add((field, val.try_into()?));
                }
                None => {
                    return Err(SqlError::DomainError(DomainError::ScalarFieldNotFound {
                        name: columns[i].clone(),
                        model: String::from("unspecified"),
                    }))
                }
            }
        }
    }

    if model_projection.scalar_length() == record_projection.len() {
        Ok(record_projection)
    } else {
        Err(SqlError::DomainError(DomainError::ConversionFailure(
            "ResultSet".to_owned(),
            "RecordProjection".to_owned(),
        )))
    }
}

pub trait RecordProjectionExt {
    fn db_values<'a>(&self) -> Vec<Value<'a>>;
}

impl RecordProjectionExt for RecordProjection {
    fn db_values<'a>(&self) -> Vec<Value<'a>> {
        self.pairs.iter().map(|(f, v)| f.value(v.clone())).collect()
    }
}
