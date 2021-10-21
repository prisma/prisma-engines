#[derive(Debug, Default)]
pub struct InternalDataModelBuilder {
    pub models: Vec<ModelBuilder>,
    pub relations: Vec<RelationBuilder>,
    pub enums: Vec<InternalEnum>,
    pub composite_types: Vec<CompositeTypeRef>,
}

impl InternalDataModelBuilder {
    pub fn build(self, db_name: String) -> InternalDataModelRef {
        let internal_data_model = Arc::new(InternalDataModel {
            models: OnceCell::new(),
            composite_types: OnceCell::new(),
            relations: OnceCell::new(),
            relation_fields: OnceCell::new(),
            db_name,
            enums: self.enums.into_iter().map(Arc::new).collect(),
        });

        let composite_types = self.composite_types.into_iter().map(|builder| builder.build());

        let models = self
            .models
            .into_iter()
            .map(|mt| mt.build(Arc::downgrade(&internal_data_model)))
            .collect();

        internal_data_model.models.set(models).unwrap();

        let relations = self
            .relations
            .into_iter()
            .map(|rt| rt.build(Arc::downgrade(&internal_data_model)))
            .collect();

        internal_data_model.relations.set(relations).unwrap();
        internal_data_model.finalize();
        internal_data_model
    }
}
