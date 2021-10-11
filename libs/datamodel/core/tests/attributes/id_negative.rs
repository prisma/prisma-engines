use crate::attributes::with_postgres_provider;
use crate::common::*;
use indoc::indoc;

#[test]
fn id_should_error_if_the_field_is_not_required() {
    let dml = indoc! {r#"
        model Model {
          id Int? @id
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@id": Fields that are marked as id must be required.[0m
          [1;94m-->[0m  [4mschema.prisma:2[0m
        [1;94m   | [0m
        [1;94m 1 | [0mmodel Model {
        [1;94m 2 | [0m  id Int? @[1;91mid[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn id_should_error_multiple_ids_are_provided() {
    let dml = indoc! {r#"
        model Model {
          id         Int      @id
          internalId String   @id @default(uuid())
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "Model": At most one field must be marked as the id field with the `@id` attribute.[0m
          [1;94m-->[0m  [4mschema.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0m[1;91mmodel Model {[0m
        [1;94m 2 | [0m  id         Int      @id
        [1;94m 3 | [0m  internalId String   @id @default(uuid())
        [1;94m 4 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn id_must_error_when_single_and_multi_field_id_is_used() {
    let dml = indoc! {r#"
        model Model {
          id         Int      @id
          b          String

          @@id([id,b])
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "Model": Each model must have at most one id criteria. You can't have `@id` and `@@id` at the same time.[0m
          [1;94m-->[0m  [4mschema.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0m[1;91mmodel Model {[0m
        [1;94m 2 | [0m  id         Int      @id
        [1;94m 3 | [0m  b          String
        [1;94m 4 | [0m
        [1;94m 5 | [0m  @@id([id,b])
        [1;94m 6 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn id_must_error_when_multi_field_is_referring_to_undefined_fields() {
    let dml = indoc! {r#"
        model Model {
          a String
          b String

          @@id([a,c])
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "Model": The multi field id declaration refers to the unknown fields c.[0m
          [1;94m-->[0m  [4mschema.prisma:5[0m
        [1;94m   | [0m
        [1;94m 4 | [0m
        [1;94m 5 | [0m  @@id([1;91m[a,c][0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn relation_fields_as_part_of_compound_id_must_error() {
    let dml = indoc! {r#"
        model User {
          name           String
          identification Identification @relation(references:[id])

          @@id([name, identification])
        }

        model Identification {
          id Int @id
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": The id definition refers to the relation fields identification. ID definitions must reference only scalar fields.[0m
          [1;94m-->[0m  [4mschema.prisma:5[0m
        [1;94m   | [0m
        [1;94m 4 | [0m
        [1;94m 5 | [0m  @@[1;91mid([name, identification])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_when_multi_field_is_referring_fields_that_are_not_required() {
    let dml = indoc! {r#"
        model Model {
          a String
          b String?
          c String?

          @@id([a,b,c])
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "Model": The id definition refers to the optional fields b, c. ID definitions must reference only required fields.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m
        [1;94m 6 | [0m  @@[1;91mid([a,b,c])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn stringified_field_names_in_id_return_nice_error() {
    let dml = indoc! {r#"
        model User {
          firstName String
          lastName  String

          @@id(["firstName", "lastName"])
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mExpected a constant literal value, but received string value `"firstName"`.[0m
          [1;94m-->[0m  [4mschema.prisma:5[0m
        [1;94m   | [0m
        [1;94m 4 | [0m
        [1;94m 5 | [0m  @@id([[1;91m"firstName"[0m, "lastName"])
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn relation_field_as_id_must_error() {
    let dml = indoc! {r#"
        model User {
          identification Identification @relation(references:[id]) @id
        }

        model Identification {
          id Int @id
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@id": The field `identification` is a relation field and cannot be marked with `@id`. Only scalar fields can be declared as id.[0m
          [1;94m-->[0m  [4mschema.prisma:2[0m
        [1;94m   | [0m
        [1;94m 1 | [0mmodel User {
        [1;94m 2 | [0m  identification Identification @relation(references:[id]) @[1;91mid[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn invalid_name_for_compound_id_must_error() {
    let dml = with_postgres_provider(indoc! {r#"
        model User {
          name           String
          identification Int

          @@id([name, identification], name: "Test.User")
        }
    "#});

    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": The `name` property within the `@@id` attribute only allows for the following characters: `_a-zA-Z0-9`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  @@[1;91mid([name, identification], name: "Test.User")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn mapped_id_must_error_on_mysql() {
    let dml = indoc! {r#"
        datasource test {
          provider = "mysql"
          url = "mysql://root:prisma@127.0.0.1:3309/NoNamedPKsOnMysql"
        }

        model User {
          name           String
          identification Int

          @@id([name, identification], map: "NotSupportedByProvider")
        }

        model User1 {
          name           String @id(map: "NotSupportedByProvider")
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": You defined a database name for the primary key on the model. This is not supported by the provider.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m
        [1;94m 6 | [0m[1;91mmodel User {[0m
        [1;94m 7 | [0m  name           String
        [1;94m 8 | [0m  identification Int
        [1;94m 9 | [0m
        [1;94m10 | [0m  @@id([name, identification], map: "NotSupportedByProvider")
        [1;94m11 | [0m}
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating model "User1": You defined a database name for the primary key on the model. This is not supported by the provider.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m
        [1;94m13 | [0m[1;91mmodel User1 {[0m
        [1;94m14 | [0m  name           String @id(map: "NotSupportedByProvider")
        [1;94m15 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn mapped_id_must_error_on_sqlite() {
    let dml = indoc! {r#"
        datasource test {
          provider = "sqlite"
          url = "file://...."
        }

        model User {
          name           String
          identification Int

          @@id([name, identification], map: "NotSupportedByProvider")
        }

        model User1 {
          name           String @id(map: "NotSupportedByProvider")
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": You defined a database name for the primary key on the model. This is not supported by the provider.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m
        [1;94m 6 | [0m[1;91mmodel User {[0m
        [1;94m 7 | [0m  name           String
        [1;94m 8 | [0m  identification Int
        [1;94m 9 | [0m
        [1;94m10 | [0m  @@id([name, identification], map: "NotSupportedByProvider")
        [1;94m11 | [0m}
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating model "User1": You defined a database name for the primary key on the model. This is not supported by the provider.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m
        [1;94m13 | [0m[1;91mmodel User1 {[0m
        [1;94m14 | [0m  name           String @id(map: "NotSupportedByProvider")
        [1;94m15 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn naming_id_to_a_field_name_should_error() {
    let dml = with_postgres_provider(indoc! {r#"
        model User {
          used           Int
          name           String
          identification Int

          @@id([name, identification], name: "used")
        }
    "#});

    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": The custom name `used` specified for the `@@id` attribute is already used as a name for a field. Please choose a different name.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m    }
        [1;94m11 | [0m[1;91mmodel User {[0m
        [1;94m12 | [0m  used           Int
        [1;94m13 | [0m  name           String
        [1;94m14 | [0m  identification Int
        [1;94m15 | [0m
        [1;94m16 | [0m  @@id([name, identification], name: "used")
        [1;94m17 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn mapping_id_with_a_name_that_is_too_long_should_error() {
    let dml = with_postgres_provider(indoc! {r#"
        model User {
          name           String
          identification Int

          @@id([name, identification], map: "IfYouAreGoingToPickTheNameYourselfYouShouldReallyPickSomethingShortAndSweetInsteadOfASuperLongNameViolatingLengthLimits")
        }

        model User1 {
          name           String @id(map: "IfYouAreGoingToPickTheNameYourselfYouShouldReallyPickSomethingShortAndSweetInsteadOfASuperLongNameViolatingLengthLimitsHereAsWell")
          identification Int
        }
    "#});

    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": The constraint name 'IfYouAreGoingToPickTheNameYourselfYouShouldReallyPickSomethingShortAndSweetInsteadOfASuperLongNameViolatingLengthLimits' specified in the `map` argument for the `@@id` constraint is too long for your chosen provider. The maximum allowed length is 63 bytes.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  @@[1;91mid([name, identification], map: "IfYouAreGoingToPickTheNameYourselfYouShouldReallyPickSomethingShortAndSweetInsteadOfASuperLongNameViolatingLengthLimits")[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating model "User1": The constraint name 'IfYouAreGoingToPickTheNameYourselfYouShouldReallyPickSomethingShortAndSweetInsteadOfASuperLongNameViolatingLengthLimitsHereAsWell' specified in the `map` argument for the `@id` constraint is too long for your chosen provider. The maximum allowed length is 63 bytes.[0m
          [1;94m-->[0m  [4mschema.prisma:19[0m
        [1;94m   | [0m
        [1;94m18 | [0mmodel User1 {
        [1;94m19 | [0m  name           String @[1;91mid(map: "IfYouAreGoingToPickTheNameYourselfYouShouldReallyPickSomethingShortAndSweetInsteadOfASuperLongNameViolatingLengthLimitsHereAsWell")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn name_on_field_level_id_should_error() {
    let dml = with_postgres_provider(indoc! {r#"
        model User {
          invalid           Int @id(name: "THIS SHOULD BE MAP INSTEAD")
        }
    "#});

    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNo such argument.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mmodel User {
        [1;94m12 | [0m  invalid           Int @id([1;91mname: "THIS SHOULD BE MAP INSTEAD"[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn bytes_should_not_be_allowed_as_id_on_sql_server() {
    let dml = indoc! {r#"
        datasource db {
            provider = "sqlserver"
            url      = "sqlserver://"
        }

        generator client {
            provider = "prisma-client-js"
        }

        model A {
            id Bytes @id
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();
    let expected = expect![[r#"
        [1;91merror[0m: [1mInvalid model: Using Bytes type is not allowed in the model's id..[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m
        [1;94m10 | [0m[1;91mmodel A {[0m
        [1;94m11 | [0m    id Bytes @id
        [1;94m12 | [0m}
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn primary_key_and_foreign_key_names_cannot_clash() {
    let dml = indoc! { r#"
        datasource test {
          provider = "postgresql"
          url = "postgresql://"
        }

        model A {
            id Int @id(map: "foo") 
            bId Int
            b   B  @relation(fields: [bId], references: [id], map: "foo")
        }
        
        model B {
            id Int @id(map: "bar")
            as A[]
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@id": The given constraint name `foo` has to be unique in the following namespace: on model `A` for primary key, indexes, unique constraints and foreign keys. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0mmodel A {
        [1;94m 7 | [0m    id Int @[1;91mid(map: "foo")[0m 
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": The given constraint name `foo` has to be unique in the following namespace: on model `A` for primary key, indexes, unique constraints and foreign keys. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m    bId Int
        [1;94m 9 | [0m    b   B  @relation(fields: [bId], references: [id], [1;91mmap: "foo"[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}
