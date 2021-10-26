use crate::common::*;

#[test]
fn indexes_on_relation_fields_must_error() {
    let dml = indoc! {r#"
        model User {
          id               Int @id
          identificationId Int

          identification   Identification @relation(fields: [identificationId], references:[id])

          @@index([identification])
        }

        model Identification {
          id Int @id
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": The index definition refers to the relation fields identification. Index definitions must reference only scalar fields. Did you mean `@@index([identificationId])`?[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m
        [1;94m 7 | [0m  @@[1;91mindex([identification])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_when_unknown_fields_are_used() {
    let dml = indoc! {r#"
        model User {
          id Int @id

          @@index([foo,bar])
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": The index definition refers to the unknown fields foo, bar.[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m
        [1;94m 4 | [0m  @@[1;91mindex([foo,bar])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn stringified_field_names_in_index_return_nice_error() {
    let dml = indoc! {r#"
        model User {
          id        Int    @id
          firstName String
          lastName  String

          @@index(["firstName", "lastName"])
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mExpected a constant literal value, but received string value `"firstName"`.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m
        [1;94m 6 | [0m  @@index([[1;91m"firstName"[0m, "lastName"])
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}
