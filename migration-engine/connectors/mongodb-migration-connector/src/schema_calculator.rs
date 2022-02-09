use datamodel::{
    datamodel_connector::walker_ext_traits::*,
    parser_database::{IndexType, SortOrder},
    ValidatedSchema,
};
use mongodb_schema_describer::{IndexField, IndexFieldProperty, MongoSchema};

/// Datamodel -> MongoSchema
pub(crate) fn calculate(datamodel: &ValidatedSchema) -> MongoSchema {
    let mut schema = MongoSchema::default();
    let connector = mongodb_datamodel_connector::MongoDbDatamodelConnector;

    for model in datamodel.db.walk_models() {
        let collection_id = schema.push_collection(model.database_name().to_owned());

        for index in model.indexes() {
            let name = index.constraint_name(&connector);
            let fields = index
                .scalar_field_attributes()
                .map(|field| (field.as_scalar_field().database_name(), field.sort_order()))
                .map(|(name, sort_order)| {
                    let property = match sort_order {
                        Some(SortOrder::Desc) => IndexFieldProperty::Descending,
                        None if index.is_fulltext() => IndexFieldProperty::Text,
                        _ => IndexFieldProperty::Ascending,
                    };

                    IndexField {
                        name: name.to_string(),
                        property,
                    }
                })
                .collect();

            let r#type = match index.index_type() {
                IndexType::Unique => mongodb_schema_describer::IndexType::Unique,
                IndexType::Normal => mongodb_schema_describer::IndexType::Normal,
                IndexType::Fulltext => mongodb_schema_describer::IndexType::Fulltext,
            };

            schema.push_index(collection_id, name.into_owned(), r#type, fields);
        }
    }

    schema
}
