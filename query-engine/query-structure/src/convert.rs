use crate::InternalDataModel;
use crosstarget_utils::psl::ValidatedSchema;
use std::sync::Arc;

pub fn convert(schema: Arc<ValidatedSchema>) -> InternalDataModel {
    InternalDataModel { schema }
}
