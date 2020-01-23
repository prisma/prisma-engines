use crate::{error::DomainError, ModelIdentifier, RecordIdentifier};
use quaint::connector::ResultSet;
use std::{convert::TryFrom, sync::Arc};

impl TryFrom<(&ModelIdentifier, ResultSet)> for RecordIdentifier {
    type Error = DomainError;

    fn try_from(pair: (&ModelIdentifier, ResultSet)) -> crate::Result<Self> {
        let (id, result_set) = pair;

        let columns: Vec<String> = result_set.columns().iter().map(|c| c.to_string()).collect();
        let mut record_id = RecordIdentifier::default();

        for row in result_set.into_iter() {
            for (i, val) in row.into_iter().enumerate() {
                match id.map_db_name(columns[i].as_str()) {
                    Some(field) => {
                        record_id.add((Arc::clone(field), val.into()));
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

        if id.len() == record_id.len() {
            Ok(record_id)
        } else {
            Err(DomainError::ConversionFailure("ResultSet", "RecordIdentifier"))
        }
    }
}
