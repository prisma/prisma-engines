use crate::common::*;
use psl::diagnostics::{DatamodelWarning, Span};

#[test]
fn nice_warning_for_deprecated_generator_preview_feature() {
    let schema = r#"
    generator client {
        provider = "prisma-client"
        previewFeatures = ["middlewares"]
    }
    "#;

    let res = psl::parse_configuration(schema).unwrap();

    res.warnings
        .assert_is(DatamodelWarning::new_preview_feature_is_stabilized(
            "middlewares",
            Span::new(85, 100, psl_core::parser_database::FileId::ZERO),
        ));
}
