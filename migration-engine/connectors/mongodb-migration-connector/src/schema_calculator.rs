use datamodel::Datamodel;
use mongodb_schema_describer::{IndexField, IndexFieldProperty, MongoSchema};

/// Datamodel -> MongoSchema
pub(crate) fn calculate(datamodel: &Datamodel) -> MongoSchema {
    let mut schema = MongoSchema::default();

    for model in datamodel.models() {
        let collection_id = schema.push_collection(model.database_name.as_ref().unwrap_or(&model.name).clone());

        for index in &model.indices {
            let name = index.db_name.clone().expect("unnamed index");
            let fields = index
                .fields
                .iter()
                .map(|field| {
                    let sf = model.find_scalar_field(&field.name).unwrap();
                    (sf.db_name(), field.sort_order)
                })
                .map(|(name, sort_order)| {
                    let property = match sort_order {
                        Some(datamodel::SortOrder::Desc) => IndexFieldProperty::Descending,
                        None if index.is_fulltext() => IndexFieldProperty::Text,
                        _ => IndexFieldProperty::Ascending,
                    };

                    IndexField {
                        name: name.to_string(),
                        property,
                    }
                })
                .collect();

            let r#type = match index.tpe {
                datamodel::IndexType::Unique => mongodb_schema_describer::IndexType::Unique,
                datamodel::IndexType::Normal => mongodb_schema_describer::IndexType::Normal,
                datamodel::IndexType::Fulltext => mongodb_schema_describer::IndexType::Fulltext,
            };

            schema.push_index(collection_id, name, r#type, fields);
        }
    }

    schema
}
