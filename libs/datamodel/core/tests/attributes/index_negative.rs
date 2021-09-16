use crate::attributes::with_named_constraints;
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
fn empty_index_names_are_rejected() {
    let dml = indoc! {r#"
        model User {
          id        Int    @id
          firstName String
          lastName  String

          @@index([firstName,lastName], name: "")
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The `name` argument cannot be an empty string.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m
        [1;94m 6 | [0m  @@[1;91mindex([firstName,lastName], name: "")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn empty_unique_index_names_are_rejected() {
    let dml = indoc! {r#"
        model User {
          id        Int    @id
          firstName String
          lastName  String

          @@unique([firstName,lastName], name: "")
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": The `name` argument cannot be an empty string.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m
        [1;94m 6 | [0m  @@[1;91munique([firstName,lastName], name: "")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn multiple_indexes_with_same_name_are_not_supported_by_sqlite() {
    let dml = indoc! {r#"
        datasource sqlite {
          provider = "sqlite"
          url = "sqlite://asdlj"
        }

        model User {
          id         Int @id
          neighborId Int

          @@index([id], name: "MyIndexName")
        }

        model Post {
          id Int @id
          optionId Int

          @@index([id], name: "MyIndexName")
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe index name `MyIndexName` is declared multiple times. With the current connector index names have to be globally unique.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m
        [1;94m17 | [0m  @@[1;91mindex([id], name: "MyIndexName")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn multiple_indexes_with_same_name_are_not_supported_by_postgres() {
    let dml = indoc! {r#"
        datasource postgres {
          provider = "postgres"
          url = "postgres://asdlj"
        }

        model User {
          id         Int @id
          neighborId Int

          @@index([id], name: "MyIndexName")
        }

        model Post {
          id Int @id
          optionId Int

          @@index([id], name: "MyIndexName")
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe index name `MyIndexName` is declared multiple times. With the current connector index names have to be globally unique.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m
        [1;94m17 | [0m  @@[1;91mindex([id], name: "MyIndexName")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn unique_indexes_with_same_name_are_not_supported_by_postgres() {
    let dml = indoc! {r#"
        datasource postgres {
          provider = "postgres"
          url = "postgres://asdlj"
        }

        model User {
          id         Int @id
          neighborId Int

          @@index([id], map: "MyIndexName")
        }

        model Post {
          id Int @id
          optionId Int

          @@unique([id], map: "MyIndexName")
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe index name `MyIndexName` is declared multiple times. With the current connector index names have to be globally unique.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m
        [1;94m17 | [0m  @@[1;91munique([id], map: "MyIndexName")[0m
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

#[test]
fn having_both_the_map_and_name_argument_must_be_rejected() {
    let dml = with_named_constraints(indoc! {r#"
        model User {
          id        Int    @id
          firstName String
          lastName  String

          @@index([firstName,lastName], name: "BOTH MAP AND NAME IS NOT OK", map: "MyIndexName")
        }
    "#});

    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The `@@index` attribute accepts the `name` argument as an alias for the `map` argument for legacy reasons. It does not accept both though. Please use the `map` argument to specify the database name of the index.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m
        [1;94m16 | [0m  @@[1;91mindex([firstName,lastName], name: "BOTH MAP AND NAME IS NOT OK", map: "MyIndexName")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}
