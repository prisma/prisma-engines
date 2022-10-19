use psl_core::{
    datamodel_connector::{walker_ext_traits::ScalarFieldWalkerExt, Connector},
    diagnostics::Diagnostics,
    parser_database::walkers::{IndexWalker, PrimaryKeyWalker},
};

const LENGTH_GUIDE: &str = " Please use the `length` argument to the field in the index definition to allow this.";

const NATIVE_TYPES_THAT_CAN_NOT_BE_USED_IN_KEY_SPECIFICATION: &[&str] = &[
    super::TEXT_TYPE_NAME,
    super::LONG_TEXT_TYPE_NAME,
    super::MEDIUM_TEXT_TYPE_NAME,
    super::TINY_TEXT_TYPE_NAME,
    super::BLOB_TYPE_NAME,
    super::TINY_BLOB_TYPE_NAME,
    super::MEDIUM_BLOB_TYPE_NAME,
    super::LONG_BLOB_TYPE_NAME,
];

pub(crate) fn field_types_can_be_used_in_an_index(
    connector: &dyn Connector,
    index: IndexWalker<'_>,
    errors: &mut Diagnostics,
) {
    for field in index.scalar_field_attributes() {
        let native_type = match field.as_index_field().native_type_instance(connector) {
            Some(native_type) => native_type,
            None => continue,
        };
        let (native_type_name, _) = connector.native_type_to_parts(&native_type);

        if !NATIVE_TYPES_THAT_CAN_NOT_BE_USED_IN_KEY_SPECIFICATION.contains(&native_type_name) {
            continue;
        }

        // Length defined, so we allow the index.
        if field.length().is_some() {
            continue;
        }

        if index.is_fulltext() {
            continue;
        }

        let error = if index.is_unique() {
            connector
                .native_instance_error(&native_type)
                .new_incompatible_native_type_with_unique(LENGTH_GUIDE, index.ast_attribute().span)
        } else {
            connector
                .native_instance_error(&native_type)
                .new_incompatible_native_type_with_index(LENGTH_GUIDE, index.ast_attribute().span)
        };

        errors.push_error(error);

        break;
    }
}

pub(crate) fn field_types_can_be_used_in_a_primary_key(
    connector: &dyn Connector,
    primary_key: PrimaryKeyWalker<'_>,
    errors: &mut Diagnostics,
) {
    for field in primary_key.scalar_field_attributes() {
        let native_type = match field.as_index_field().native_type_instance(connector) {
            Some(native_type) => native_type,
            None => continue,
        };
        let (native_type_name, _) = connector.native_type_to_parts(&native_type);

        if !NATIVE_TYPES_THAT_CAN_NOT_BE_USED_IN_KEY_SPECIFICATION.contains(&native_type_name) {
            continue;
        }

        if field.length().is_some() {
            continue;
        }

        let span = primary_key.ast_attribute().span;

        let error = connector
            .native_instance_error(&native_type)
            .new_incompatible_native_type_with_id(LENGTH_GUIDE, span);

        errors.push_error(error);

        break;
    }
}
