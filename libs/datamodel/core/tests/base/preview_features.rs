use crate::common::*;
use datamodel::ast::Span;
use datamodel::error::DatamodelError;

#[test]
fn must_allow_transaction_as_model_name_if_preview_feature_is_not_set() {
    let dml = r#"
    model Transaction {
        id Int @id
    }
    "#;

    // must not error
    let _ = parse(dml);
}

#[test]
fn must_disallow_transaction_as_model_name_if_preview_feature_is_set() {
    let dml = r#"
    generator js {
        provider = "js"
        previewFeatures = ["transactionApi"]
    }
    
    model Transaction {
        id Int @id
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_is(DatamodelError::new_model_validation_error(
        "The model name `Transaction` is invalid. It is a reserved name. Please change it. Read more at https://pris.ly/d/naming-models",
        "Transaction",
        Span::new(104, 148),
    ));
}
