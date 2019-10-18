#![allow(non_snake_case)]
#![allow(unused)]
mod test_harness;
use pretty_assertions::{assert_eq, assert_ne};
use test_harness::*;

#[test]
fn unapply_must_work() {
    test_each_connector(|test_setup, api| {
        let dm1 = r#"
            model Test {
                id String @id @default(cuid())
                field String
            }
        "#;

        let result1 = infer_and_apply(test_setup, api, &dm1).sql_schema;
        assert_eq!(result1.table_bang("Test").column("field").is_some(), true);

        let dm2 = r#"
            model Test {
                id String @id @default(cuid())
            }
        "#;

        let result2 = infer_and_apply(test_setup, api, &dm2).sql_schema;
        assert_eq!(result2.table_bang("Test").column("field").is_some(), false);

        let result3 = unapply_migration(test_setup, api);
        assert_eq!(result1, result3);

        // reapply the migration again
        let result4 = infer_and_apply(test_setup, api, &dm2).sql_schema;
        assert_eq!(result2, result4);
    });
}
