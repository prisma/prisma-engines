use crate::common::*;
use datamodel::{ast, diagnostics::DatamodelError};

#[test]
fn should_fail_on_native_type_text_with_unique_attribute() {
    let dml = r#"
        datasource db {
          provider = "mysql"
          url = "mysql://"
          previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id     Int    @id
            bigInt String @db.Text @unique
        }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_connector_error(
        "Native type Text can not be unique in MySQL.",
        ast::Span::new(199, 230),
    ));
}

#[test]
fn should_fail_on_native_type_long_text_with_unique_attribute() {
    let dml = r#"
        datasource db {
          provider = "mysql"
          url = "mysql://"
          previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id     Int    @id
            bigInt String @db.LongText @unique
        }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_connector_error(
        "Native type LongText can not be unique in MySQL.",
        ast::Span::new(199, 234),
    ));
}

#[test]
fn should_fail_on_native_type_medium_text_with_unique_attribute() {
    let dml = r#"
        datasource db {
          provider = "mysql"
          url = "mysql://"
          previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id     Int    @id
            bigInt String @db.MediumText @unique
        }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_connector_error(
        "Native type MediumText can not be unique in MySQL.",
        ast::Span::new(199, 236),
    ));
}

#[test]
fn should_fail_on_native_type_tiny_text_with_unique_attribute() {
    let dml = r#"
        datasource db {
          provider = "mysql"
          url = "mysql://"
          previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id     Int    @id
            bigInt String @db.TinyText @unique
        }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_connector_error(
        "Native type TinyText can not be unique in MySQL.",
        ast::Span::new(199, 234),
    ));
}

#[test]
fn should_fail_on_native_type_blob_with_unique_attribute() {
    let dml = r#"
        datasource db {
          provider = "mysql"
          url = "mysql://"
          previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id     Int    @id
            bigInt Bytes @db.Blob @unique
        }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_connector_error(
        "Native type Blob can not be unique in MySQL.",
        ast::Span::new(199, 229),
    ));
}

#[test]
fn should_fail_on_native_type_tiny_blob_with_unique_attribute() {
    let dml = r#"
        datasource db {
          provider = "mysql"
          url = "mysql://"
          previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id     Int    @id
            bigInt Bytes @db.TinyBlob @unique
        }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_connector_error(
        "Native type TinyBlob can not be unique in MySQL.",
        ast::Span::new(199, 233),
    ));
}

#[test]
fn should_fail_on_native_type_medium_blob_with_unique_attribute() {
    let dml = r#"
        datasource db {
          provider = "mysql"
          url = "mysql://"
          previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id     Int    @id
            bigInt Bytes @db.MediumBlob @unique
        }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_connector_error(
        "Native type MediumBlob can not be unique in MySQL.",
        ast::Span::new(199, 235),
    ));
}

#[test]
fn should_fail_on_native_type_long_blob_with_unique_attribute() {
    let dml = r#"
        datasource db {
          provider = "mysql"
          url = "mysql://"
          previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id     Int    @id
            bigInt Bytes @db.LongBlob @unique
        }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_connector_error(
        "Native type LongBlob can not be unique in MySQL.",
        ast::Span::new(199, 233),
    ));
}
