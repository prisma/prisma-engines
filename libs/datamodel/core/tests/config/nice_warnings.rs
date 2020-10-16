use crate::common::*;
use datamodel::ast::Span;
use datamodel::diagnostics::DatamodelWarning;

#[test]
fn nice_warning_for_deprecated_datasource_preview_feature() {
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
