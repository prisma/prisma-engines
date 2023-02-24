use crate::{InternalDataModel, InternalDataModelRef};
use std::sync::Arc;

pub fn convert(schema: Arc<psl::ValidatedSchema>) -> InternalDataModelRef {
    Arc::new(InternalDataModel { schema })
}
