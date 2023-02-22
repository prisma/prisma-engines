use super::IndexBuilder;
use crate::{Fields, InternalDataModelWeakRef, Model, ModelRef};
use once_cell::sync::OnceCell;
use psl::schema_ast::ast;
use std::sync::Arc;

#[derive(Debug)]
pub struct ModelBuilder {
    pub id: ast::ModelId,
    pub name: String,
    pub manifestation: Option<String>,
    pub indexes: Vec<IndexBuilder>,
    pub dml_model: dml::Model,
}

impl ModelBuilder {
    pub fn build(self, internal_data_model: InternalDataModelWeakRef, schema: &psl::ValidatedSchema) -> ModelRef {
        let model = Arc::new(Model {
            id: self.id,
            name: self.name,
            manifestation: self.manifestation,
            fields: OnceCell::new(),
            indexes: OnceCell::new(),
            primary_identifier: OnceCell::new(),
            dml_model: self.dml_model,
            internal_data_model,
        });

        let primary_key = {
            let dm = model.internal_data_model.upgrade().unwrap();
            schema.db.walk(model.id).primary_key().map(|pk| crate::pk::PrimaryKey {
                alias: pk.name().map(ToOwned::to_owned),
                fields: pk
                    .fields()
                    .map(|f| dm.clone().zip(crate::ScalarFieldId::InModel(f.id)))
                    .collect(),
            })
        };
        let fields = Fields::new(Arc::downgrade(&model), primary_key);

        let indexes = self.indexes.into_iter().map(|i| i.build(&fields.scalar())).collect();

        // The model is created here and fields WILL BE UNSET before now!
        model.fields.set(fields).unwrap();
        model.indexes.set(indexes).unwrap();
        model
    }
}
