use psl_core::{
    diagnostics::{DatamodelError, Diagnostics},
    parser_database::{
        walkers::{IndexWalker, ModelWalker},
        IndexAlgorithm, ScalarType,
    },
};

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
