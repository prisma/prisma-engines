use mongodb_schema_describer::{IndexField, IndexFieldProperty, MongoSchema};
use psl::{
    builtin_connectors::MONGODB,
    datamodel_connector::walker_ext_traits::*,
    parser_database::{IndexType, SortOrder},
    ValidatedSchema,
};

/// Datamodel -> MongoSchema
pub(crate) fn calculate(datamodel: &ValidatedSchema) -> MongoSchema {
    let mut schema = MongoSchema::default();

    for model in datamodel.db.walk_models() {
        let collection_id = schema.push_collection(model.database_name().to_owned(), false, false);

        for index in model.indexes() {
            let name = index.constraint_name(MONGODB);

            let fields = index
                .scalar_field_attributes()
                .map(|field| {
                    let path = field
                        .as_mapped_path_to_indexed_field()
                        .into_iter()
                        .map(|(f, _)| f.to_owned())
                        .collect::<Vec<_>>()
                        .join(".");

                    (path, field.sort_order())
                })
                .map(|(name, sort_order)| {
                    let property = match sort_order {
                        Some(SortOrder::Desc) => IndexFieldProperty::Descending,
                        None if index.is_fulltext() => IndexFieldProperty::Text,
                        _ => IndexFieldProperty::Ascending,
                    };

                    IndexField { name, property }
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
