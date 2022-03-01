use crate::common::*;
use datamodel::ScalarType;

#[test]
fn unique_attribute() {
    let dml = r#"
        model Test {
            id Int @id
            unique String @unique
        }
    "#;

    let schema = parse(dml);
    let test_model = schema.assert_has_model("Test");

    test_model
        .assert_has_scalar_field("id")
        .assert_base_type(&ScalarType::Int)
        .assert_is_id(test_model);

    assert!(!test_model.field_is_unique("id"));

    test_model
        .assert_has_scalar_field("unique")
        .assert_base_type(&ScalarType::String);

    assert!(test_model.field_is_unique("unique"));
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
        [1;91merror[0m: [1mAttribute "@unique" is defined twice.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id String @id
        [1;94m 3 | [0m  unique String @[1;91munique[0m @unique
        [1;94m   | [0m
        [1;91merror[0m: [1mAttribute "@unique" is defined twice.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id String @id
        [1;94m 3 | [0m  unique String @unique @[1;91munique[0m
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&datamodel::parse_schema(dml).map(drop).unwrap_err());
}
