use crate::common::*;
use datamodel::ast::Span;
use datamodel::diagnostics::DatamodelError;

#[test]
fn disallow_ignore_missing_from_model_without_fields() {
    let dml = r#"
    model ModelNoFields {
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_model_validation_error(
        "Each model must have at least one unique criteria that has only required fields. Either mark a single field with `@id`, `@unique` or add a multi field criterion with `@@id([])` or `@@unique([])` to the model.",
        "ModelNoFields",
        Span::new(5, 32),
    ));
}

#[test]
fn disallow_ignore_missing_from_model_without_id() {
    let dml = r#"
    model ModelNoId {
        text String
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_model_validation_error(
        "Each model must have at least one unique criteria that has only required fields. Either mark a single field with `@id`, `@unique` or add a multi field criterion with `@@id([])` or `@@unique([])` to the model.",
        "ModelNoId",
        Span::new(5, 48),
    ));
}

#[test]
fn disallow_ignore_missing_from_model_with_optional_id() {
    let dml = r#"
    model ModelOptionalId {
        text String? @id
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_are(&[DatamodelError::new_attribute_validation_error(
        "Fields that are marked as id must be required.",
        "id",
        Span::new(51, 53),
    )]);
}

#[test]
fn disallow_ignore_missing_from_model_with_unsupported_id() {
    let dml = r#"
    model ModelUnsupportedId {
        text Unsupported("something") @id
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_model_validation_error(
        "Each model must have at least one unique criteria that has only required fields. Either mark a single field with `@id`, `@unique` or add a multi field criterion with `@@id([])` or `@@unique([])` to the model. The following unique criterias were not considered as they contain fields that are not required:\n- text",
        "ModelUnsupportedId",
        Span::new(5, 79),
    ));
}

#[test]
fn disallow_ignore_missing_from_model_with_compound_unsupported_id() {
    let dml = r#"
    model ModelCompoundUnsupportedId {
        text Unsupported("something")
        int  Int

        @@id([text, int])
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_model_validation_error(
        "Each model must have at least one unique criteria that has only required fields. Either mark a single field with `@id`, `@unique` or add a multi field criterion with `@@id([])` or `@@unique([])` to the model. The following unique criterias were not considered as they contain fields that are not required:\n- text, int",
        "ModelCompoundUnsupportedId",
        Span::new(5, 127),
    ));
}

#[test]
fn disallow_ignore_on_models_with_relations_pointing_to_them() {
    let dml = r#"
    model ModelValidC {
      id Int @id
      d  Int
      rel_d  ModelValidD @relation(fields: d, references: id) //ignore here is missing
    }

    model ModelValidD {
      id Int @id
      rel_c  ModelValidC[]

      @@ignore
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "The relation field `rel_d` on Model `ModelValidC` must specify the `@ignore` attribute, because the model ModelValidD it is pointing to is marked ignored.",
        "ignore",
        Span::new(61, 142),
    ));
}

#[test]
fn disallow_ignore_on_models_with_back_relations_pointing_to_them() {
    let dml = r#"
    model ModelValidA {
      id Int @id
      b  Int
      rel_b  ModelValidB @relation(fields: b, references: id)

      @@ignore
    }

    model ModelValidB {
      id Int @id
      rel_a  ModelValidA[] //ignore is missing here
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "The relation field `rel_a` on Model `ModelValidB` must specify the `@ignore` attribute, because the model ModelValidA it is pointing to is marked ignored.",
        "ignore",
        Span::new(187, 233),
    ));
}

#[test]
fn disallow_ignore_on_unsupported() {
    let dml = r#"
    model ModelValidA {
      id Int @id
      b  Unsupported("something") @ignore
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "Fields of type `Unsupported` cannot take an `@ignore` attribute. They are already treated as ignored by the client due to their type.",
        "ignore",
        Span::new(77, 83),
    ));
}

#[test]
fn disallow_ignore_on_ignored_model() {
    let dml = r#"
    model ModelValidA {
      id Int @id
      b  String @ignore

      @@ignore
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "Fields on an already ignored Model do not need an `@ignore` annotation.",
        "ignore",
        Span::new(48, 66),
    ));
}
