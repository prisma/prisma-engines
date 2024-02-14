use crate::InternalDataModel;
use std::sync::Arc;

pub fn convert(schema: Arc<dyn psl::ValidSchema>) -> InternalDataModel {
    InternalDataModel { schema }
}
