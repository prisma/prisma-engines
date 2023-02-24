use crate::{builders, InternalDataModel, InternalDataModelRef};
use once_cell::sync::OnceCell;
use std::sync::Arc;

pub fn convert(schema: Arc<psl::ValidatedSchema>) -> InternalDataModelRef {
    let datamodel = dml::lift(&schema);

    let models = builders::model_builders(&datamodel, &schema);

    let internal_data_model = Arc::new(InternalDataModel {
        models: OnceCell::new(),
        schema,
    });

    let models = models
        .into_iter()
        .map(|mt| mt.build(Arc::downgrade(&internal_data_model), &internal_data_model.schema))
        .collect();

    internal_data_model.models.set(models).unwrap();
    internal_data_model
}
