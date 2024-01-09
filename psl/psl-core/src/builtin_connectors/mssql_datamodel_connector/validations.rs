use diagnostics::Span;

use super::MsSqlType;
use crate::{
    datamodel_connector::{walker_ext_traits::ScalarFieldWalkerExt, Connector, NativeTypeInstance},
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

pub(crate) fn validate_model(
    connector: &dyn Connector,
    model: parser_database::walkers::ModelWalker<'_>,
    errors: &mut Diagnostics,
) {
    for index in model.indexes() {
        index_uses_correct_field_types(connector, index, errors);
    }

    if let Some(pk) = model.primary_key() {
        primary_key_uses_correct_field_types(connector, pk, errors);
    }
}

pub(crate) fn validate_native_type_arguments(
    connector: &dyn Connector,
    native_type: &NativeTypeInstance,
    span: Span,
    errors: &mut Diagnostics,
) {
    use crate::builtin_connectors::MsSqlTypeParameter::*;
    use MsSqlType::*;

    let r#type: &MsSqlType = native_type.downcast_ref();
    let error = connector.native_instance_error(native_type);

    match r#type {
        Decimal(Some((precision, scale))) if scale > precision => {
            errors.push_error(error.new_scale_larger_than_precision_error(span));
        }
        Decimal(Some((prec, _))) if *prec == 0 || *prec > 38 => {
            errors.push_error(error.new_argument_m_out_of_range_error("Precision can range from 1 to 38.", span));
        }
        Decimal(Some((_, scale))) if *scale > 38 => {
            errors.push_error(error.new_argument_m_out_of_range_error("Scale can range from 0 to 38.", span))
        }
        Float(Some(bits)) if *bits == 0 || *bits > 53 => {
            errors.push_error(error.new_argument_m_out_of_range_error("Bits can range from 1 to 53.", span))
        }
        NVarChar(Some(Number(p))) if *p > 4000 => errors.push_error(error.new_argument_m_out_of_range_error(
            "Length can range from 1 to 4000. For larger sizes, use the `Max` variant.",
            span,
        )),
        VarChar(Some(Number(p))) | VarBinary(Some(Number(p))) if *p > 8000 => {
            errors.push_error(error.new_argument_m_out_of_range_error(
                r#"Length can range from 1 to 8000. For larger sizes, use the `Max` variant."#,
                span,
            ))
        }
        NChar(Some(p)) if *p > 4000 => {
            errors.push_error(error.new_argument_m_out_of_range_error("Length can range from 1 to 4000.", span))
        }
        Char(Some(p)) | Binary(Some(p)) if *p > 8000 => {
            errors.push_error(error.new_argument_m_out_of_range_error("Length can range from 1 to 8000.", span))
        }
        _ => (),
    }
}
