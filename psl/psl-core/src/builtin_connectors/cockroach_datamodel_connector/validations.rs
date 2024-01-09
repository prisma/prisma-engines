use schema_ast::ast;

use crate::{
    datamodel_connector::{Connector, NativeTypeInstance},
    diagnostics::{DatamodelError, Diagnostics},
    parser_database::{
        walkers::{IndexWalker, ModelWalker},
        IndexAlgorithm, ScalarType,
    },
};

use super::{CockroachType, SequenceFunction};

/// Validating the correct usage of GIN indices.
pub(super) fn inverted_index_validations(index: IndexWalker<'_>, errors: &mut Diagnostics) {
    let algo = index.algorithm().unwrap_or(IndexAlgorithm::BTree);

    if !algo.is_gin() {
        return;
    }

    let field_count = index.scalar_field_attributes().len();

    for (i, field) in index.scalar_field_attributes().enumerate() {
        let r#type = field.as_index_field().scalar_field_type();

        if field.operator_class().is_some() {
            let msg = "Custom operator classes are not supported with the current connector.";

            errors.push_error(DatamodelError::new_attribute_validation_error(
                msg,
                index.attribute_name(),
                index.ast_attribute().span,
            ));

            return;
        }

        if !algo.supports_field_type(field.as_index_field()) {
            let name = field.as_index_field().name();
            let msg = format!("The {algo} index type does not support the type of the field `{name}`.");

            errors.push_error(DatamodelError::new_attribute_validation_error(
                &msg,
                index.attribute_name(),
                index.ast_attribute().span,
            ));
        }

        if r#type.is_json() && i < (field_count - 1) {
            let msg = "A `Json` column is only allowed as the last column of an inverted index.";
            errors.push_error(DatamodelError::new_attribute_validation_error(
                msg,
                index.attribute_name(),
                index.ast_attribute().span,
            ));
        }
    }
}

pub(super) fn autoincrement_validations(model: ModelWalker<'_>, errors: &mut Diagnostics) {
    let autoincrement_defaults_on_int = model
        .scalar_fields()
        .filter_map(|sf| sf.default_value().map(|d| (sf, d)))
        .filter(|(sf, d)| d.is_autoincrement() && matches!(sf.scalar_type(), Some(ScalarType::Int)));

    for (_scalar_field, default_value) in autoincrement_defaults_on_int {
        errors.push_error(DatamodelError::new_attribute_validation_error(
            "The `autoincrement()` default function is defined only on BigInt fields on CockroachDB. Use sequence() if you want an autoincrementing Int field.",
            "default",
            default_value.ast_attribute().span,
        ));
    }
}

pub(crate) fn validate_model(model: ModelWalker<'_>, diagnostics: &mut Diagnostics) {
    autoincrement_validations(model, diagnostics);

    for index in model.indexes() {
        inverted_index_validations(index, diagnostics);
    }
}

pub(crate) fn validate_scalar_field_unknown_default_functions(
    db: &parser_database::ParserDatabase,
    diagnostics: &mut Diagnostics,
) {
    for d in db.walk_scalar_field_defaults_with_unknown_function() {
        let (func_name, args, span) = d.value().as_function().unwrap();
        match func_name {
            "sequence" => {
                SequenceFunction::validate(args, diagnostics);
            }
            _ => diagnostics.push_error(DatamodelError::new_default_unknown_function(func_name, span)),
        }
    }
}

pub(crate) fn validate_native_type_arguments(
    connector: &dyn Connector,
    native_type_instance: &NativeTypeInstance,
    span: ast::Span,
    errors: &mut Diagnostics,
) {
    let native_type: &CockroachType = native_type_instance.downcast_ref();
    let error = connector.native_instance_error(native_type_instance);

    match native_type {
        CockroachType::Decimal(Some((precision, scale))) if scale > precision => {
            errors.push_error(error.new_scale_larger_than_precision_error(span))
        }
        CockroachType::Decimal(Some((prec, _))) if *prec > 1000 || *prec == 0 => errors.push_error(
            error.new_argument_m_out_of_range_error("Precision must be positive with a maximum value of 1000.", span),
        ),
        CockroachType::Bit(Some(0)) | CockroachType::VarBit(Some(0)) => {
            errors.push_error(error.new_argument_m_out_of_range_error("M must be a positive integer.", span))
        }
        CockroachType::Timestamp(Some(p))
        | CockroachType::Timestamptz(Some(p))
        | CockroachType::Time(Some(p))
        | CockroachType::Timetz(Some(p))
            if *p > 6 =>
        {
            errors.push_error(error.new_argument_m_out_of_range_error("M can range from 0 to 6.", span))
        }
        _ => (),
    }
}
