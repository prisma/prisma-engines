use super::MsSqlType;
use psl_core::{
    datamodel_connector::{walker_ext_traits::ScalarFieldWalkerExt, Connector},
    diagnostics::{DatamodelError, Diagnostics},
    parser_database::{
        walkers::{IndexWalker, PrimaryKeyWalker},
        ScalarType,
    },
};

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

        let r#type: &MsSqlType = native_type.downcast_ref();

        if !super::heap_allocated_types().contains(r#type) {
            continue;
        }

        let error = connector.native_instance_error(&native_type);

        if index.is_unique() {
            errors.push_error(error.new_incompatible_native_type_with_unique("", index.ast_attribute().span))
        } else {
            errors.push_error(error.new_incompatible_native_type_with_index("", index.ast_attribute().span))
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
            let r#type: &MsSqlType = native_type.downcast_ref();

            if super::heap_allocated_types().contains(r#type) {
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
