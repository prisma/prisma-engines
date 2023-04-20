use crate::common::*;
use psl::parser_database::ScalarType;

#[test]
fn unique_attribute() {
    let dml = r#"
        model Test {
            id Int @id
            unique String @unique
        }
    "#;

    let schema = psl::parse_schema(dml).unwrap();
    let model = schema.assert_has_model("Test");

    model
        .assert_has_scalar_field("id")
        .assert_scalar_type(ScalarType::Int)
        .assert_not_single_field_unique()
        .assert_is_single_field_id();

    model
        .assert_has_scalar_field("unique")
        .assert_scalar_type(ScalarType::String)
        .assert_is_single_field_unique();
}

#[test]
fn duplicate_attributes_should_error() {
    let dml = indoc! {r#"
        model Test {
          id String @id
          unique String @unique @unique
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mAttribute "@unique" can only be defined once.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id String @id
        [1;94m 3 | [0m  unique String [1;91m@unique [0m@unique
        [1;94m   | [0m
        [1;91merror[0m: [1mAttribute "@unique" can only be defined once.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id String @id
        [1;94m 3 | [0m  unique String @unique [1;91m@unique[0m
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}
