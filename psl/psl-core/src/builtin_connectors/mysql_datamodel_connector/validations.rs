use crate::{
    datamodel_connector::{walker_ext_traits::ScalarFieldWalkerExt, Connector, NativeTypeInstance, RelationMode},
    diagnostics::Diagnostics,
    diagnostics::{DatamodelWarning, Span},
    parser_database::{
        ast::WithSpan,
        walkers::{IndexWalker, PrimaryKeyWalker, RelationFieldWalker},
        ReferentialAction,
    },
};
use diagnostics::DatamodelError;
use indoc::formatdoc;
use parser_database::{walkers, ScalarType};

use super::MySqlType;

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

pub(crate) fn uses_native_referential_action_set_default(
    connector: &dyn Connector,
    field: RelationFieldWalker<'_>,
    diagnostics: &mut Diagnostics,
) {
    let get_span = |referential_action_type: &str| -> Span {
        field
            .ast_field()
            .span_for_argument("relation", referential_action_type)
            .unwrap_or_else(|| field.ast_field().span())
    };

    let warning_msg = || {
        formatdoc!(
            r#"
            {connector_name} does not actually support the `{set_default}` referential action, so using it may result in unexpected errors.
            Read more at https://pris.ly/d/mysql-set-default
            "#,
            connector_name = connector.name(),
            set_default = ReferentialAction::SetDefault.as_str(),
        ).replace('\n', " ")
    };

    if let Some(ReferentialAction::SetDefault) = field.explicit_on_delete() {
        let span = get_span("onDelete");
        diagnostics.push_warning(DatamodelWarning::new(warning_msg(), span));
    }

    if let Some(ReferentialAction::SetDefault) = field.explicit_on_update() {
        let span = get_span("onUpdate");
        diagnostics.push_warning(DatamodelWarning::new(warning_msg(), span));
    }
}

pub(crate) fn validate_model(
    connector: &dyn Connector,
    model: walkers::ModelWalker<'_>,
    relation_mode: RelationMode,
    errors: &mut Diagnostics,
) {
    for index in model.indexes() {
        field_types_can_be_used_in_an_index(connector, index, errors);
    }

    if let Some(pk) = model.primary_key() {
        field_types_can_be_used_in_a_primary_key(connector, pk, errors);
    }

    if relation_mode.uses_foreign_keys() {
        for field in model.relation_fields() {
            uses_native_referential_action_set_default(connector, field, errors);
        }
    }
}

pub(crate) fn validate_enum(r#enum: walkers::EnumWalker<'_>, diagnostics: &mut Diagnostics) {
    if let Some((_, span)) = r#enum.schema() {
        diagnostics.push_error(DatamodelError::new_static(
            "MySQL enums do not belong to a schema.",
            span,
        ));
    }
}

pub(crate) fn validate_native_type_arguments(
    connector: &dyn Connector,
    native_type_instance: &NativeTypeInstance,
    scalar_type: &ScalarType,
    span: Span,
    errors: &mut Diagnostics,
) {
    use MySqlType::*;
    let native_type: &MySqlType = native_type_instance.downcast_ref();
    let error = connector.native_instance_error(native_type_instance);

    match native_type {
        Decimal(Some((precision, scale))) if scale > precision => {
            errors.push_error(error.new_scale_larger_than_precision_error(span))
        }
        Decimal(Some((precision, _))) if *precision > 65 => {
            errors.push_error(error.new_argument_m_out_of_range_error("Precision can range from 1 to 65.", span))
        }
        Decimal(Some((_, scale))) if *scale > 30 => {
            errors.push_error(error.new_argument_m_out_of_range_error("Scale can range from 0 to 30.", span))
        }
        Bit(length) if *length == 0 || *length > 64 => {
            errors.push_error(error.new_argument_m_out_of_range_error("M can range from 1 to 64.", span))
        }
        Char(length) if *length > 255 => {
            errors.push_error(error.new_argument_m_out_of_range_error("M can range from 0 to 255.", span))
        }
        VarChar(length) if *length > 65535 => {
            errors.push_error(error.new_argument_m_out_of_range_error("M can range from 0 to 65,535.", span))
        }
        Bit(n) if *n > 1 && matches!(scalar_type, ScalarType::Boolean) => {
            errors.push_error(error.new_argument_m_out_of_range_error("only Bit(1) can be used as Boolean.", span))
        }
        _ => (),
    }
}
