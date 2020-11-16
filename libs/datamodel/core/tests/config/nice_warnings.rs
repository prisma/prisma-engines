use crate::common::*;
use datamodel::ast::Span;
use datamodel::diagnostics::DatamodelWarning;

#[test]
fn nice_warning_for_deprecated_generator_preview_feature() {
    let schema = r#"
    generator client {
        provider = "prisma-client-js"
        previewFeatures = ["middlewares"]
    }
    "#;

    let res = datamodel::parse_configuration(schema).unwrap();

    res.warnings
        .assert_is(DatamodelWarning::new_deprecated_preview_feature_warning(
            "middlewares",
            Span::new(88, 103),
        ));
}

#[test]
fn nice_warning_for_provider_array_deprecation() {
    let schema = r#"datasource db {
  provider = ["sqlite", "postgres"]
  url = "postgres://"
}
"#;

    let res = parse_with_diagnostics(schema);

    res.warnings
        .assert_is(DatamodelWarning::new_deprecated_provider_array_warning(Span::new(
            29, 51,
        )));
}

#[test]
fn nice_warning_for_provider_array_deprecation_on_single_element_in_array() {
    let schema = r#"datasource db {
  provider = ["postgres"]
  url = "postgres://"
}
"#;

    let res = parse_with_diagnostics(schema);

    res.warnings
        .assert_is(DatamodelWarning::new_deprecated_provider_array_warning(Span::new(
            29, 41,
        )));
}
