use crate::{error::DomainError, ModelProjection, RecordProjection};
use quaint::connector::ResultSet;
use std::convert::TryFrom;

impl TryFrom<(&ModelProjection, ResultSet)> for RecordProjection {
    type Error = DomainError;

    fn try_from(pair: (&ModelProjection, ResultSet)) -> crate::Result<Self> {
        let (model_projection, result_set) = pair;

        let columns: Vec<String> = result_set.columns().iter().map(|c| c.to_string()).collect();
        let mut record_projection = RecordProjection::default();

        for row in result_set.into_iter() {
            for (i, val) in row.into_iter().enumerate() {
                match model_projection.map_db_name(columns[i].as_str()) {
                    Some(field) => {
                        record_projection.add((field, val.into()));
                    }
                    None => {
                        return Err(DomainError::ScalarFieldNotFound {
                            name: columns[i].clone(),
                            model: String::from("unspecified"),
                        })
                    }
                }
            }

            break;
        }

        if model_projection.len() == record_projection.len() {
            Ok(record_projection)
        } else {
            Err(DomainError::ConversionFailure(
                "ResultSet".to_owned(),
                "RecordProjection".to_owned(),
            ))
        }
    }
}
