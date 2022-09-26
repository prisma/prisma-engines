use crate::common::*;
use psl::diagnostics::{DatamodelWarning, Span};

#[test]
fn nice_warning_for_deprecated_generator_preview_feature() {
    let schema = r#"
    generator client {
        provider = "prisma-client-js"
        previewFeatures = ["middlewares"]
    }
    "#;

    let res = psl::parse_configuration(schema).unwrap();

    res.warnings.assert_is(DatamodelWarning::new_feature_deprecated(
        "middlewares",
        Span::new(88, 103),
    ));
}
