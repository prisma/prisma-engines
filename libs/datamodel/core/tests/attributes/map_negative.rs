use crate::{common::*, with_header, Provider};

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
        [1;94m 4 | [0m  relationField  Foo @relation(fields: [fooId], references: [id]) [1;91m@map("custom_name")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn mongodb_field_mapping_cannot_start_with_a_dollar_sign() {
    let dml = indoc! {r#"
        model Foo {
          id    Int    @id @map("_id")
          field String @map("$field")
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@map": The field name cannot start with a `$` character[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id    Int    @id @map("_id")
        [1;94m13 | [0m  field String [1;91m@map("$field")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn mongodb_field_mapping_cannot_contain_periods() {
    let dml = indoc! {r#"
        model Foo {
          id    Int    @id @map("_id")
          field String @map("field.schwield")
        }
    "#};

    let schema = with_header(dml, Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@map": The field name cannot contain a `.` character[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id    Int    @id @map("_id")
        [1;94m13 | [0m  field String [1;91m@map("field.schwield")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}
