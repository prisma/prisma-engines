use crate::common::*;
use indoc::indoc;

#[test]
fn map_must_error_for_relation_fields() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          fooId Int
          relationField  Foo @relation(fields: [fooId], references: [id]) @map("custom_name")
        }

        model Foo {
          id Int @id
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@map": The attribute `@map` cannot be used on relation fields.[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m  fooId Int
        [1;94m 4 | [0m  relationField  Foo @relation(fields: [fooId], references: [id]) @[1;91mmap("custom_name")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}
