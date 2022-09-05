use super::{FieldBuilder, IndexBuilder, PrimaryKeyBuilder};
use crate::{CompositeTypeRef, Fields, InternalDataModelWeakRef, Model, ModelRef};
use once_cell::sync::OnceCell;
use std::sync::Arc;

#[derive(Debug)]
pub struct ModelBuilder {
    pub name: String,
    pub fields: Vec<FieldBuilder>,
    pub manifestation: Option<String>,
    pub primary_key: Option<PrimaryKeyBuilder>,
    pub indexes: Vec<IndexBuilder>,
    pub supports_create_operation: bool,
    pub dml_model: psl::dml::Model,
}

impl ModelBuilder {
    pub fn build(
        self,
        internal_data_model: InternalDataModelWeakRef,
        composite_types: &[CompositeTypeRef],
    ) -> ModelRef {
        let model = Arc::new(Model {
            name: self.name,
            manifestation: self.manifestation,
            fields: OnceCell::new(),
            indexes: OnceCell::new(),
            primary_identifier: OnceCell::new(),
            dml_model: self.dml_model,
            internal_data_model,
            supports_create_operation: self.supports_create_operation,
        });

        let all_fields: Vec<_> = self
            .fields
            .into_iter()
            .map(|ft| ft.build(Arc::downgrade(&model).into(), composite_types))
            .collect();

        let pk = self.primary_key.map(|pk| pk.build(&all_fields));
        let fields = Fields::new(all_fields, Arc::downgrade(&model), pk);

        let indexes = self.indexes.into_iter().map(|i| i.build(&fields.scalar())).collect();

        // The model is created here and fields WILL BE UNSET before now!
        model.fields.set(fields).unwrap();
        model.indexes.set(indexes).unwrap();
        model
    }
}
