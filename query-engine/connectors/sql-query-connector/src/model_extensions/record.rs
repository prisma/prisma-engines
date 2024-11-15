use crate::{value::to_prisma_value, SqlError};
use quaint::connector::ResultSet;
use query_structure::{DomainError, ModelProjection, SelectionResult};

pub fn try_convert(model_projection: &ModelProjection, result_set: ResultSet) -> crate::Result<SelectionResult> {
    let columns: Vec<String> = result_set.columns().iter().map(|c| c.to_string()).collect();
    let mut record_projection = SelectionResult::default();

    if let Some(row) = result_set.into_iter().next() {
        for (i, val) in row.into_iter().enumerate() {
            match model_projection.map_db_name(columns[i].as_str()) {
                Some(field) => {
                    record_projection.add((field, to_prisma_value(val)?));
                }
                None => {
                    return Err(SqlError::DomainError(DomainError::ScalarFieldNotFound {
                        name: columns[i].clone(),
                        container_type: "model",
                        container_name: String::from("unspecified"),
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
