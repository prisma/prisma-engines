use super::{CompositeTypeBuilder, IndexBuilder, ModelBuilder};
use crate::IndexType;
use dml::{self, Datamodel, WithDatabaseName};

pub(crate) fn model_builders(datamodel: &Datamodel, schema: &psl::ValidatedSchema) -> Vec<ModelBuilder> {
    datamodel
        .models()
        .filter(|model| !schema.db.walk(model.id).is_ignored())
        .filter(|model| {
            let walker = schema.db.walk(model.id);

            walker
                .primary_key()
                .map(|pk| pk.fields())
                .into_iter()
                .flatten()
                .all(|f| !f.is_unsupported())
                || walker
                    .indexes()
                    .filter(|idx| idx.is_unique())
                    .any(|idx| idx.fields().all(|f| !f.is_unsupported()))
        })
        .map(|model| ModelBuilder {
            id: model.id,
            name: model.name.clone(),
            manifestation: model.database_name().map(|s| s.to_owned()),
            indexes: index_builders(model),
            dml_model: model.clone(),
        })
        .collect()
}

fn index_builders(model: &dml::Model) -> Vec<IndexBuilder> {
    model
        .indices
        .iter()
        .filter(|i| i.fields.len() > 1) // @@unique for 1 field are transformed to is_unique instead
        .filter(|i| i.fields.iter().all(|f| f.path.len() <= 1)) // TODO: we do not take indices with composite fields for now
        .map(|i| IndexBuilder {
            name: i.name.clone(),
            fields: i
                .fields
                .clone()
                .into_iter()
                .map(|mut f| f.path.pop().unwrap().0)
                .collect(),
            typ: match i.tpe {
                dml::IndexType::Unique => IndexType::Unique,
                dml::IndexType::Normal => IndexType::Normal,
                // TODO: When introducing the indexes in QE, change this.
                dml::IndexType::Fulltext => IndexType::Normal,
            },
        })
        .collect()
}

pub(crate) fn composite_type_builders(datamodel: &Datamodel) -> Vec<CompositeTypeBuilder> {
    datamodel
        .composite_types
        .iter()
        .map(|ct| CompositeTypeBuilder {
            id: ct.id,
            name: ct.name.clone(),
        })
        .collect()
}
