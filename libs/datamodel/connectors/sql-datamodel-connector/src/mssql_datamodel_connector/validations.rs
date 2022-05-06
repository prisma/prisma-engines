use datamodel_connector::{
    parser_database::walkers::{IndexWalker, PrimaryKeyWalker},
    walker_ext_traits::ScalarFieldWalkerExt,
    Connector, DatamodelError, Diagnostics, ScalarType, Span,
};
use native_types::MsSqlType;

pub(crate) fn index_uses_correct_field_types(
    connector: &dyn Connector,
    index: IndexWalker<'_>,
    errors: &mut Diagnostics,
) {
    for field in index.fields() {
        let native_type = match field.native_type_instance(connector) {
            Some(native_type) => native_type,
            None => continue,
        };

        let r#type: MsSqlType = serde_json::from_value(native_type.serialized_native_type.clone()).unwrap();

        if !super::heap_allocated_types().contains(&r#type) {
            continue;
        }

        let span = index.ast_attribute().map(|attr| attr.span).unwrap_or_else(Span::empty);

        let error = connector.native_instance_error(&native_type);

        if index.is_unique() {
            errors.push_error(error.new_incompatible_native_type_with_unique("", span))
        } else {
            errors.push_error(error.new_incompatible_native_type_with_index("", span))
        };

        break;
    }
}

pub(crate) fn primary_key_uses_correct_field_types(
    connector: &dyn Connector,
    pk: PrimaryKeyWalker<'_>,
    errors: &mut Diagnostics,
) {
    for field in pk.fields() {
        let span = pk.ast_attribute().span;

        if let Some(native_type) = field.native_type_instance(connector) {
            let r#type: MsSqlType = serde_json::from_value(native_type.serialized_native_type.clone()).unwrap();

            if super::heap_allocated_types().contains(&r#type) {
                let error = connector
                    .native_instance_error(&native_type)
                    .new_incompatible_native_type_with_id("", span);

                errors.push_error(error);

                break;
            }
        };

        if let Some(ScalarType::Bytes) = field.scalar_type() {
            errors.push_error(DatamodelError::new_invalid_model_error(
                "Using Bytes type is not allowed in the model's id.",
                span,
            ));

            break;
        }
    }
}
