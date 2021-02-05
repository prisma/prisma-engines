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

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "Cannot set a default value on a relation field.",
        "default",
        Span::new(53, 64),
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

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "Cannot set a default value on a relation field.",
        "default",
        Span::new(53, 64),
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

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "Cannot set a default value on a relation field.",
        "default",
        Span::new(53, 64),
    ));
}

#[test]
fn disallow_ignore_missing_from_model_with_unsupported_id() {
    let dml = r#"   
    model ModelUnsupportedId {
        text Unsupported("something") @id
    }   
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "Cannot set a default value on a relation field.",
        "default",
        Span::new(53, 64),
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

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "Cannot set a default value on a relation field.",
        "default",
        Span::new(53, 64),
    ));
}

#[test]
fn disallow_ignore_on_models_with_relations_pointing_to_them() {
    let dml = r#"   
    model ModelValidC {
      id Int @id
      d  Int
      rel_d  ModelValidD @relation(fields:[d]) //ignore here is missing
    }
    
    model ModelValidD {
      id Int @id
      rel_c  ModelValidC[] 
      
      @@ignore
    }   
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "Cannot set a default value on a relation field.",
        "default",
        Span::new(53, 64),
    ));
}

#[test]
fn disallow_ignore_on_models_with_back_relations_pointing_to_them() {
    let dml = r#"
    model ModelValidA {
      id Int @id
      b  Int
      rel_b  ModelValidB @relation(fields:[b]) 
     
      @@ignore
    }
    
    model ModelValidB {
      id Int @id
      rel_a  ModelValidA[] //ignore is missing here
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_attribute_validation_error(
        "Cannot set a default value on a relation field.",
        "default",
        Span::new(53, 64),
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
        "Cannot set a default value on a relation field.",
        "default",
        Span::new(53, 64),
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
        "Cannot set a default value on a relation field.",
        "default",
        Span::new(53, 64),
    ));
}
