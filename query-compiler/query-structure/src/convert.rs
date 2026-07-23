use crate::InternalDataModel;
use std::sync::Arc;

pub fn convert(schema: Arc<psl::ValidatedSchema>) -> InternalDataModel {
    InternalDataModel { schema }
}
