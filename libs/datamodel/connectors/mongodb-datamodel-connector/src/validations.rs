use datamodel_connector::{
    parser_database::{
        ast::{WithName, WithSpan},
        walkers::{IndexWalker, ModelWalker, PrimaryKeyWalker, ScalarFieldWalker},
    },
    DatamodelError, Diagnostics,
};

/// If `@default(auto())`, then also `@db.ObjectId`
pub(super) fn objectid_type_required_with_auto_attribute(field: ScalarFieldWalker<'_>, errors: &mut Diagnostics) {
    if !field.default_value().map(|val| val.is_auto()).unwrap_or(false) {
        return;
    }

    if matches!(field.raw_native_type().map(|t| t.1), Some("ObjectId")) {
        return;
    }

    let err = DatamodelError::new_field_validation_error(
        "MongoDB `@default(auto())` fields must have `ObjectId` native type.",
        field.model().name(),
        field.name(),
        field.ast_field().span,
    );

    errors.push_error(err);
}

/// If `@default(auto())`, then also `@id`.
pub(super) fn auto_attribute_must_be_an_id(field: ScalarFieldWalker<'_>, errors: &mut Diagnostics) {
    if field.is_single_pk() || field.is_part_of_a_compound_pk() {
        return;
    }

    if !field.default_value().map(|val| val.is_auto()).unwrap_or(false) {
        return;
    }

    let err = DatamodelError::new_field_validation_error(
        "MongoDB `@default(auto())` fields must have the `@id` attribute.",
        field.model().name(),
        field.name(),
        field.ast_field().span,
    );

    errors.push_error(err);
}

/// `@default(dbgenerated())` is only for SQL connectors.
pub(super) fn dbgenerated_attribute_is_not_allowed(field: ScalarFieldWalker<'_>, errors: &mut Diagnostics) {
    if !field.default_value().map(|val| val.is_dbgenerated()).unwrap_or(false) {
        return;
    }

    let err = DatamodelError::new_field_validation_error(
        "The `dbgenerated()` function is not allowed with MongoDB. Please use `auto()` instead.",
        field.model().name(),
        field.name(),
        field.ast_field().span,
    );
    errors.push_error(err);
}

/// The _id name check is superfluous because it's not a valid schema field at the moment.
pub(super) fn id_field_must_have_a_correct_mapped_name(pk: PrimaryKeyWalker<'_>, errors: &mut Diagnostics) {
    if pk.fields().len() > 1 {
        return;
    }

    let field = match pk.fields().next() {
        Some(field) => field,
        None => return,
    };

    if field.name() == "_id" || field.mapped_name() == Some("_id") {
        return;
    }

    let error = match field.mapped_name() {
        Some(name) => {
            let msg = format!("MongoDB model IDs must have a @map(\"_id\") annotation, found @map(\"{name}\").",);

            DatamodelError::new_field_validation_error(&msg, field.model().name(), field.name(), field.ast_field().span)
        }
        None => DatamodelError::new_field_validation_error(
            "MongoDB model IDs must have a @map(\"_id\") annotations.",
            field.model().name(),
            field.name(),
            field.ast_field().span,
        ),
    };

    errors.push_error(error);
}

/// Must define one field as an `@id`.
pub(super) fn id_must_be_defined(model: ModelWalker<'_>, errors: &mut Diagnostics) {
    if model.primary_key().is_some() {
        return;
    }

    errors.push_error(DatamodelError::new_invalid_model_error(
        "MongoDB models require exactly one identity field annotated with @id",
        model.ast_model().span,
    ));
}

/// We can define only one index with the same parameters.
pub(crate) fn index_is_not_defined_multiple_times_to_same_fields(index: IndexWalker<'_>, errors: &mut Diagnostics) {
    let attr = if let Some(attribute) = index.ast_attribute() {
        attribute
    } else {
        return;
    };

    let hits = index
        .model()
        .indexes()
        .filter(|i| !i.is_implicit())
        .filter(|i| i.attribute_id() != index.attribute_id())
        .filter(|i| i.contains_exactly_the_fields(index.scalar_field_attributes()))
        .count();

    if hits == 0 {
        return;
    }

    let attr_name = attr.name();

    errors.push_error(DatamodelError::new_attribute_validation_error(
        "Index already exists in the model.",
        &format!("@@{attr_name}"),
        *attr.span(),
    ))
}

/// A field cannot have `@id` and `@unique` attributes at the same time.
pub(crate) fn unique_cannot_be_defined_to_id_field(index: IndexWalker<'_>, errors: &mut Diagnostics) {
    let attr = if let Some(attribute) = index.ast_attribute() {
        attribute
    } else {
        return;
    };

    if !index.is_unique() {
        return;
    }

    if index.fields().len() != 1 {
        return;
    }

    let field = index.fields().next().unwrap();

    if !field.is_single_pk() {
        return;
    }

    errors.push_error(DatamodelError::new_attribute_validation_error(
        "The same field cannot be an id and unique on MongoDB.",
        index.attribute_name(),
        *attr.span(),
    ));
}

/// A field name cannot contain the `.` character and it cannot start with `$`.
pub(crate) fn field_name_uses_valid_characters(field: ScalarFieldWalker<'_>, errors: &mut Diagnostics) {
    let name = match field.mapped_name() {
        Some(name) => name,
        None => return,
    };

    let span = field.ast_field().span_for_attribute("map").unwrap();

    if name.starts_with('$') {
        errors.push_error(DatamodelError::new_attribute_validation_error(
            "The field name cannot start with a `$` character",
            "@map",
            span,
        ));
    }

    if name.contains('.') {
        errors.push_error(DatamodelError::new_attribute_validation_error(
            "The field name cannot contain a `.` character",
            "@map",
            span,
        ));
    }
}
