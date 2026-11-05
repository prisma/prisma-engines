use crate::{
    datamodel_connector::{Connector, NativeTypeInstance, walker_ext_traits::ScalarFieldWalkerExt},
    diagnostics::Diagnostics,
    parser_database::walkers::{IndexWalker, PrimaryKeyWalker, ScalarFieldAttributeWalker},
};

use super::KingbaseOracleType;

const TYPES_NOT_ALLOWED_IN_KEYS: &[&str] = &["Json", "Xml"];

fn native_type_for_key_field(
    connector: &dyn Connector,
    field: ScalarFieldAttributeWalker<'_>,
) -> Option<NativeTypeInstance> {
    let field = field.as_index_field();

    field.native_type_instance(connector).or_else(|| {
        field
            .scalar_field_type()
            .is_json()
            .then(|| NativeTypeInstance::new::<KingbaseOracleType>(KingbaseOracleType::Json))
    })
}

pub(crate) fn field_types_can_be_used_in_an_index(
    connector: &dyn Connector,
    index: IndexWalker<'_>,
    errors: &mut Diagnostics,
) {
    for field in index.scalar_field_attributes() {
        let Some(native_type) = native_type_for_key_field(connector, field) else {
            continue;
        };
        let (native_type_name, _) = connector.native_type_to_parts(&native_type);

        if !TYPES_NOT_ALLOWED_IN_KEYS.contains(&native_type_name) {
            continue;
        }

        let error = if index.is_unique() {
            connector
                .native_instance_error(&native_type)
                .new_incompatible_native_type_with_unique("", index.ast_attribute().span)
        } else {
            connector
                .native_instance_error(&native_type)
                .new_incompatible_native_type_with_index("", index.ast_attribute().span)
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
        let Some(native_type) = native_type_for_key_field(connector, field) else {
            continue;
        };
        let (native_type_name, _) = connector.native_type_to_parts(&native_type);

        if !TYPES_NOT_ALLOWED_IN_KEYS.contains(&native_type_name) {
            continue;
        }

        errors.push_error(
            connector
                .native_instance_error(&native_type)
                .new_incompatible_native_type_with_id("", primary_key.ast_attribute().span),
        );
        break;
    }
}
