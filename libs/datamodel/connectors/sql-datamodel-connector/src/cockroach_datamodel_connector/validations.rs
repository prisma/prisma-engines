use datamodel_connector::{
    parser_database::{walkers::IndexWalker, IndexAlgorithm},
    DatamodelError, Diagnostics,
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

        let attr = match index.ast_attribute() {
            Some(attr) => attr,
            _ => continue,
        };

        if field.operator_class().is_some() {
            let msg = "Custom operator classes are not supported with the current connector.";

            errors.push_error(DatamodelError::new_attribute_validation_error(msg, "@index", attr.span));

            return;
        }

        if !algo.supports_field_type(field.as_index_field()) {
            let name = field.as_index_field().name();
            let msg = format!("The {algo} index type does not support the type of the field `{name}`.");

            errors.push_error(DatamodelError::new_attribute_validation_error(
                &msg, "@index", attr.span,
            ));
        }

        if r#type.is_json() && i < (field_count - 1) {
            let msg = "A `Json` column is only allowed as the last column of an inverted index.";
            errors.push_error(DatamodelError::new_attribute_validation_error(msg, "@index", attr.span));
        }
    }
}
