use crate::{common::*, with_header, Provider};

#[test]
fn must_error_on_model_without_unique_criteria() {
    let dml = indoc! {r#"
        model Model {
          id String
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "Model": Each model must have at least one unique criteria that has only required fields. Either mark a single field with `@id`, `@unique` or add a multi field criterion with `@@id([])` or `@@unique([])` to the model.[0m
          [1;94m-->[0m  [4mschema.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0m[1;91mmodel Model {[0m
        [1;94m 2 | [0m  id String
        [1;94m 3 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_if_only_loose_unique_criterias_are_present() {
    let dml = indoc! {r#"
        model Model {
          id   String
          name String? @unique 
          a    String
          b    String?
          @@unique([a,b])
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "Model": Each model must have at least one unique criteria that has only required fields. Either mark a single field with `@id`, `@unique` or add a multi field criterion with `@@id([])` or `@@unique([])` to the model. The following unique criterias were not considered as they contain fields that are not required:
        - name
        - a, b[0m
          [1;94m-->[0m  [4mschema.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0m[1;91mmodel Model {[0m
        [1;94m 2 | [0m  id   String
        [1;94m 3 | [0m  name String? @unique 
        [1;94m 4 | [0m  a    String
        [1;94m 5 | [0m  b    String?
        [1;94m 6 | [0m  @@unique([a,b])
        [1;94m 7 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn multiple_unnamed_arguments_must_error() {
    let dml = indoc! {r#"
        model User {
          id        Int    @id
          firstName String
          lastName  String

          @@unique(firstName,lastName)
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": You provided multiple unnamed arguments. This is not possible. Did you forget the brackets? Did you mean `[firstName, lastName]`?[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m
        [1;94m 6 | [0m  [1;91m@@unique(firstName,lastName)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn multi_field_unique_indexes_on_relation_fields_must_error_and_give_nice_error_on_inline_side() {
    let dml = indoc! {r#"
        model User {
          id               Int @id
          identificationId Int
          identification Identification @relation(fields: [identificationId], references:[id])

          @@unique([identification])
        }

        model Identification {
          id Int @id
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": The unique index definition refers to the relation fields identification. Index definitions must reference only scalar fields. Did you mean `@@unique([identificationId])`?[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m
        [1;94m 6 | [0m  [1;91m@@unique([identification])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn multi_field_unique_indexes_on_relation_fields_must_error_and_give_nice_error_on_non_inline_side() {
    let dml = indoc! {r#"
        model User {
          id               Int @id
          identificationId Int
          identification   Identification @relation(fields: [identificationId], references:[id])
        }

        model Identification {
          id   Int @id
          user User
          @@unique([user])
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "Identification": The unique index definition refers to the relation fields user. Index definitions must reference only scalar fields.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m  user User
        [1;94m10 | [0m  [1;91m@@unique([user])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn single_field_unique_on_relation_fields_must_error_nicely_with_one_underlying_fields() {
    let dml = indoc! {r#"
        model User {
          id               Int @id
          identificationId Int
          identification Identification @relation(fields: [identificationId], references:[id]) @unique
        }

        model Identification {
          id Int @id
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": The field `identification` is a relation field and cannot be marked with `unique`. Only scalar fields can be made unique. Did you mean to put it on `identificationId`?[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m  identificationId Int
        [1;94m 4 | [0m  identification Identification @relation(fields: [identificationId], references:[id]) [1;91m@unique[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn single_field_unique_on_relation_fields_must_error_nicely_with_many_underlying_fields() {
    let dml = indoc! {r#"
        model User {
          id                Int @id
          identificationId1 Int
          identificationId2 Int
          identification Identification @relation(fields: [identificationId1, identificationId2], references:[id1, id2]) @unique
        }

        model Identification {
          id1 Int
          id2 Int
          @@id([id1, id2])
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": The field `identification` is a relation field and cannot be marked with `unique`. Only scalar fields can be made unique. Did you mean to provide `@@unique([identificationId1, identificationId2])`?[0m
          [1;94m-->[0m  [4mschema.prisma:5[0m
        [1;94m   | [0m
        [1;94m 4 | [0m  identificationId2 Int
        [1;94m 5 | [0m  identification Identification @relation(fields: [identificationId1, identificationId2], references:[id1, id2]) [1;91m@unique[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn stringified_field_names_in_unique_return_nice_error() {
    let dml = indoc! {r#"
        model User {
          id        Int    @id
          firstName String
          lastName  String

          @@unique(["firstName", "lastName"])
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mExpected a constant literal value, but received string value `"firstName"`.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m
        [1;94m 6 | [0m  @@unique([[1;91m"firstName"[0m, "lastName"])
        [1;94m   | [0m
        [1;91merror[0m: [1mExpected a constant literal value, but received string value `"lastName"`.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m
        [1;94m 6 | [0m  @@unique(["firstName", [1;91m"lastName"[0m])
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_when_unknown_fields_are_used() {
    let dml = indoc! {r#"
        model User {
          id Int @id

          @@unique([foo,bar])
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": The unique index definition refers to the unknown fields: foo, bar.[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m
        [1;94m 4 | [0m  [1;91m@@unique([foo,bar])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_when_using_the_same_field_multiple_times() {
    let dml = indoc! {r#"
        model User {
          id    Int    @id
          email String @unique

          @@unique([email, email])
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": The unique index definition refers to the field email multiple times.[0m
          [1;94m-->[0m  [4mschema.prisma:5[0m
        [1;94m   | [0m
        [1;94m 4 | [0m
        [1;94m 5 | [0m  [1;91m@@unique([email, email])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn invalid_name_for_compound_unique_must_error() {
    let dml = indoc! {r#"
        datasource test {
          provider = "mysql"
          url = "mysql://root:prisma@127.0.0.1:3309/ReproIndexNames?connection_limit=1"
        }

        model User {
          name           String            
          identification Int

          @@unique([name, identification], name: "Test.User")
        }
     "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": The `name` property within the `@@unique` attribute only allows for the following characters: `_a-zA-Z0-9`.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m
        [1;94m10 | [0m  [1;91m@@unique([name, identification], name: "Test.User")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn mapping_unique_with_a_name_that_is_too_long_should_error() {
    let dml = indoc! {r#"
        datasource test {
          provider = "mysql"
          url = "mysql://root:prisma@127.0.0.1:3309/ReproIndexNames?connection_limit=1"
        }

        model User {
          name           String            
          identification Int

          @@unique([name, identification], map: "IfYouAreGoingToPickTheNameYourselfYouShouldReallyPickSomethingShortAndSweetInsteadOfASuperLongNameViolatingLengthLimits")
        }

        model User1 {
          name           String @unique(map: "IfYouAreGoingToPickTheNameYourselfYouShouldReallyPickSomethingShortAndSweetInsteadOfASuperLongNameViolatingLengthLimitsHereAsWell")            
          identification Int      
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": The constraint name 'IfYouAreGoingToPickTheNameYourselfYouShouldReallyPickSomethingShortAndSweetInsteadOfASuperLongNameViolatingLengthLimits' specified in the `map` argument for the `@@unique` constraint is too long for your chosen provider. The maximum allowed length is 64 bytes.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m
        [1;94m10 | [0m  [1;91m@@unique([name, identification], map: "IfYouAreGoingToPickTheNameYourselfYouShouldReallyPickSomethingShortAndSweetInsteadOfASuperLongNameViolatingLengthLimits")[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating model "User1": The constraint name 'IfYouAreGoingToPickTheNameYourselfYouShouldReallyPickSomethingShortAndSweetInsteadOfASuperLongNameViolatingLengthLimitsHereAsWell' specified in the `map` argument for the `@unique` constraint is too long for your chosen provider. The maximum allowed length is 64 bytes.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0mmodel User1 {
        [1;94m14 | [0m  name           String [1;91m@unique(map: "IfYouAreGoingToPickTheNameYourselfYouShouldReallyPickSomethingShortAndSweetInsteadOfASuperLongNameViolatingLengthLimitsHereAsWell")[0m            
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn naming_unique_to_a_field_name_should_error() {
    let dml = with_header(
        indoc! {r#"
        model User {
          used           Int
          name           String            
          identification Int

          @@unique([name, identification], name: "used")
        }
    "#},
        Provider::Postgres,
        &[],
    );

    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": The custom name `used` specified for the `@@unique` attribute is already used as a name for a field. Please choose a different name.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m[1;91mmodel User {[0m
        [1;94m12 | [0m  used           Int
        [1;94m13 | [0m  name           String            
        [1;94m14 | [0m  identification Int
        [1;94m15 | [0m
        [1;94m16 | [0m  @@unique([name, identification], name: "used")
        [1;94m17 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn naming_field_level_unique_should_error() {
    let dml = with_header(
        indoc! {r#"
        model User {
          used           Int @unique(name: "INVALID ON FIELD LEVEL")
        }
    "#},
        Provider::Postgres,
        &[],
    );

    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mNo such argument.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mmodel User {
        [1;94m12 | [0m  used           Int @unique([1;91mname: "INVALID ON FIELD LEVEL"[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn duplicate_implicit_names_should_error() {
    let dml = with_header(
        indoc! {r#"
        model User {
          used           Int @unique

          @@unique([used])
        }
    "#},
        Provider::Postgres,
        &[],
    );

    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": The given constraint name `User_used_key` has to be unique in the following namespace: global for primary key, indexes and unique constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mmodel User {
        [1;94m12 | [0m  used           Int [1;91m@unique[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@@unique": The given constraint name `User_used_key` has to be unique in the following namespace: global for primary key, indexes and unique constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m
        [1;94m14 | [0m  [1;91m@@unique([used])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn duplicate_custom_names_on_same_model_should_error() {
    let dml = with_header(
        indoc! {r#"
        model A {
            id  Int
            id2 Int
            
            @@unique([id, id2], name: "foo", map: "bar")
            @@unique([id, id2], name: "foo")
        }
        
        model B {
            id  Int
            id2 Int
            
            @@unique([id, id2], name: "foo", map: "bar2")
            @@id([id, id2], name: "foo")
        }
    "#},
        Provider::Postgres,
        &[],
    );

    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@unique": The given custom name `foo` has to be unique on the model. Please provide a different name for the `name` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m    
        [1;94m15 | [0m    @@unique([id, id2], [1;91mname: "foo"[0m, map: "bar")
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@@unique": The given custom name `foo` has to be unique on the model. Please provide a different name for the `name` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m    @@unique([id, id2], name: "foo", map: "bar")
        [1;94m16 | [0m    @@unique([id, id2], [1;91mname: "foo"[0m)
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@@id": The given custom name `foo` has to be unique on the model. Please provide a different name for the `name` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:24[0m
        [1;94m   | [0m
        [1;94m23 | [0m    @@unique([id, id2], name: "foo", map: "bar2")
        [1;94m24 | [0m    @@id([id, id2], [1;91mname: "foo"[0m)
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@@unique": The given custom name `foo` has to be unique on the model. Please provide a different name for the `name` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:23[0m
        [1;94m   | [0m
        [1;94m22 | [0m    
        [1;94m23 | [0m    @@unique([id, id2], [1;91mname: "foo"[0m, map: "bar2")
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}
