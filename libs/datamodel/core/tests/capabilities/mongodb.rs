use datamodel::parse_schema;
use expect_test::expect;
use indoc::indoc;

use crate::{with_header, Provider};

#[test]
fn mongodb_supports_composite_types() {
    let schema = r#"
        type Address {
            street String
        }
    "#;

    let dml = with_header(schema, Provider::Mongo, &[]);
    assert!(parse_schema(&dml).is_ok());
}

#[test]
fn mongodb_does_not_support_autoincrement() {
    let schema = indoc! {r#"
        model User {
          id Int @id @default(autoincrement()) @map("_id")
        }
    "#};

    let dml = with_header(schema, Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The `autoincrement()` default value is used with a datasource that does not support it.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mmodel User {
        [1;94m12 | [0m  id Int @id [1;91m@default(autoincrement())[0m @map("_id")
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}
