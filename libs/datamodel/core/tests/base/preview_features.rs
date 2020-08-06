use crate::common::*;

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
