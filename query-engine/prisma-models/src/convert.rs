use crate::{builders, InternalDataModel, InternalDataModelRef};
use once_cell::sync::OnceCell;
use std::sync::Arc;

pub fn convert(schema: Arc<psl::ValidatedSchema>) -> InternalDataModelRef {
    let datamodel = dml::lift(&schema);

    let relation_placeholders = builders::relation_placeholders(&datamodel, &schema);
    let models = builders::model_builders(&datamodel, &relation_placeholders, &schema);

    let relations = builders::relation_builders(&relation_placeholders);

    let composite_types = builders::composite_type_builders(&datamodel);
    let internal_data_model = Arc::new(InternalDataModel {
        models: OnceCell::new(),
        composite_types: OnceCell::new(),
        relations: OnceCell::new(),
        relation_fields: OnceCell::new(),
        schema,
    });

    let composite_types = builders::build_composites(composite_types, Arc::downgrade(&internal_data_model));
    internal_data_model.composite_types.set(composite_types).unwrap();

    let models = models
        .into_iter()
        .map(|mt| {
            mt.build(
                Arc::downgrade(&internal_data_model),
                internal_data_model.composite_types.get().unwrap(),
            )
        })
        .collect();

    internal_data_model.models.set(models).unwrap();

    let relations = relations
        .into_iter()
        .map(|rt| rt.build(Arc::downgrade(&internal_data_model)))
        .collect();

    internal_data_model.relations.set(relations).unwrap();
    internal_data_model.finalize();
    internal_data_model
}
