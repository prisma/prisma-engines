use crate::common::*;

#[test]
fn must_error_if_default_value_for_relation_field() {
    let dml = indoc! {r#"
        model Model {
          id Int @id
          rel A @default("")
        }

        model A {
          id Int @id
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Cannot set a default value on a relation field.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  rel A @[1;91mdefault("")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_if_default_value_for_list() {
    let dml = indoc! {r#"
        datasource db {
          provider = "postgres"
          url = "postgres://"
        }

        model Model {
          id Int @id
          rel String[] @default(["hello"])
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Cannot set a default value on list field.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id Int @id
        [1;94m 8 | [0m  rel String[] @[1;91mdefault(["hello"])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_if_default_value_type_mismatch() {
    let dml = indoc! {r#"
        model Model {
          id Int @id
          rel String @default(3)
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Expected a String value, but found `3`.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  rel String @[1;91mdefault(3)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn datetime_defaults_must_be_valid_rfc3339() {
    let dml = indoc! {r#"
        model Model {
          id Int @id
          rel DateTime @default("Hugo")
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Parse error: "Hugo" is not a valid rfc3339 datetime string. (input contains invalid characters)[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  rel DateTime @default([1;91m"Hugo"[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_if_unknown_function_is_used() {
    let dml = indoc! {r#"
        model Model {
          id Int @id
          rel DateTime @default(unknown_function())
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The function `unknown_function` is not a known function. You can read about the available functions here: https://pris.ly/d/attribute-functions.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  rel DateTime @[1;91mdefault(unknown_function())[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_if_now_function_is_used_for_fields_that_are_not_datetime() {
    let dml = indoc! {r#"
        model Model {
          id  Int    @id
          foo String @default(now())
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The function `now()` cannot be used on fields of type `String`.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id  Int    @id
        [1;94m 3 | [0m  foo String @[1;91mdefault(now())[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_if_autoincrement_function_is_used_for_fields_that_are_not_int() {
    let dml = indoc! {r#"
        model Model {
          id  Int    @id
          foo String @default(autoincrement())
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The function `autoincrement()` cannot be used on fields of type `String`.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id  Int    @id
        [1;94m 3 | [0m  foo String @[1;91mdefault(autoincrement())[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_if_default_value_for_enum_is_not_valid() {
    let dml = indoc! {r#"
        model Model {
          id Int @id
          enum A @default(B)
        }

        enum A {
          A
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The defined default value `B` is not a valid value of the enum specified for the field.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  enum A @[1;91mdefault(B)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_if_using_non_id_auto_increment_on_sqlite() {
    let dml = indoc! {r#"
        datasource db1 {
          provider = "sqlite"
          url = "file://test.db"
        }

        model Model {
          id      Int @id
          non_id  Int @default(autoincrement()) @unique
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The `autoincrement()` default value is used on a non-id field even though the datasource does not support this.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id      Int @id
        [1;94m 8 | [0m  [1;91mnon_id  Int @default(autoincrement()) @unique[0m
        [1;94m 9 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_if_using_multiple_auto_increment_on_mysql() {
    let dml = indoc! {r#"
        datasource db1 {
          provider = "mysql"
          url = "mysql://"
        }

        model Model {
          id      Int @id
          non_id  Int @default(autoincrement()) @unique
          non_id2  Int @default(autoincrement()) @unique
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The `autoincrement()` default value is used multiple times on this model even though the underlying datasource only supports one instance per table.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m
        [1;94m 6 | [0m[1;91mmodel Model {[0m
        [1;94m 7 | [0m  id      Int @id
        [1;94m 8 | [0m  non_id  Int @default(autoincrement()) @unique
        [1;94m 9 | [0m  non_id2  Int @default(autoincrement()) @unique
        [1;94m10 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_if_using_non_indexed_auto_increment_on_mysql() {
    let dml = indoc! {r#"
        datasource db1 {
          provider = "mysql"
          url = "mysql://"
        }

        model Model {
          id      Int @id
          non_id  Int @default(autoincrement())
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The `autoincrement()` default value is used on a non-indexed field even though the datasource does not support this.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id      Int @id
        [1;94m 8 | [0m  [1;91mnon_id  Int @default(autoincrement())[0m
        [1;94m 9 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_on_arguments_in_autoincrement() {
    let input = indoc!(
        r#"
        model Category {
          id Int @id @default(autoincrement(name: "meow"))
        }"#
    );

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The `autoincrement` function does not take any argument. Consider changing this default to `autoincrement()`.[0m
          [1;94m-->[0m  [4mschema.prisma:2[0m
        [1;94m   | [0m
        [1;94m 1 | [0mmodel Category {
        [1;94m 2 | [0m  id Int @id @[1;91mdefault(autoincrement(name: "meow"))[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&datamodel::parse_schema(input).map(drop).unwrap_err());
}

#[test]
fn must_error_if_scalar_default_on_unsupported() {
    let dml = indoc! {r#"
        datasource db1 {
          provider = "postgresql"
          url = "postgresql://"
        }

        model Model {
          id      Int @id
          balance Unsupported("some random stuff") @default(12)
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Only @default(dbgenerated()) can be used for Unsupported types.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id      Int @id
        [1;94m 8 | [0m  balance Unsupported("some random stuff") @[1;91mdefault(12)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_if_non_string_expression_in_function_default() {
    let dml = indoc! {r#"
        model Model {
          id      Int @id
          balance Int @default(autoincrement(cuid()))
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The `autoincrement` function does not take any argument. Consider changing this default to `autoincrement()`.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id      Int @id
        [1;94m 3 | [0m  balance Int @[1;91mdefault(autoincrement(cuid()))[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_if_non_string_expression_in_function_default_2() {
    let dml = indoc! {r#"
        model Model {
          id      Int @id
          balance Int @default(dbgenerated(5))
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": `dbgenerated()` takes a single String argument[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id      Int @id
        [1;94m 3 | [0m  balance Int @[1;91mdefault(dbgenerated(5))[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_on_empty_string_in_dbgenerated() {
    let dml = indoc! {r#"
        model Model {
          id      Int @id
          balance Int @default(dbgenerated(""))
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": dbgenerated() takes either no argument, or a single nonempty string argument.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id      Int @id
        [1;94m 3 | [0m  balance Int @[1;91mdefault(dbgenerated(""))[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn dbgenerated_default_errors_must_not_cascade_into_other_errors() {
    let dml = indoc! {r#"
        datasource ds {
          provider = "mysql"
          url = "mysql://"
        }

        model User {
          id        Int    @id
          role      Bytes
          role2     Bytes @ds.VarBinary(40) @default(dbgenerated(""))

          @@unique([role2, role])
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": dbgenerated() takes either no argument, or a single nonempty string argument.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  role      Bytes
        [1;94m 9 | [0m  role2     Bytes @ds.VarBinary(40) @[1;91mdefault(dbgenerated(""))[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn named_default_constraints_should_not_work_on_non_sql_server() {
    let dml = indoc! { r#"
        datasource test {
          provider = "postgres"
          url = "postgres://"
        }

        generator js {
          provider = "prisma-client-js"
        }

        model A {
          id Int @id @default(autoincrement())
          data String @default("beeb buub", map: "meow")
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": You defined a database name for the default value of a field on the model. This is not supported by the provider.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0m  id Int @id @default(autoincrement())
        [1;94m12 | [0m  data String @[1;91mdefault("beeb buub", map: "meow")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn named_default_constraints_are_not_allowed_on_identity() {
    let dml = indoc! { r#"
        datasource test {
          provider = "sqlserver"
          url = "sqlserver://"
        }

        generator js {
          provider = "prisma-client-js"
        }

        model A {
          id Int @id @default(autoincrement(), map: "nope__nope__nope")
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Naming an autoincrement default value is not allowed.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0mmodel A {
        [1;94m11 | [0m  id Int @id @[1;91mdefault(autoincrement(), map: "nope__nope__nope")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn named_default_constraints_cannot_have_duplicate_names() {
    let dml = indoc! { r#"
        datasource test {
          provider = "sqlserver"
          url = "sqlserver://"
        }

        generator js {
          provider = "prisma-client-js"
        }

        model A {
          id Int @id @default(autoincrement())
          a  String @default("asdf", map: "reserved")
        }

        model B {
          id Int @id @default(autoincrement())
          b  String @default("asdf", map: "reserved")
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The given constraint name `reserved` has to be unique in the following namespace: global for primary keys, foreign keys and default constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0m  id Int @id @default(autoincrement())
        [1;94m12 | [0m  a  String @default("asdf", [1;91mmap: "reserved"[0m)
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@default": The given constraint name `reserved` has to be unique in the following namespace: global for primary keys, foreign keys and default constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  id Int @id @default(autoincrement())
        [1;94m17 | [0m  b  String @default("asdf", [1;91mmap: "reserved"[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn named_default_constraints_cannot_clash_with_pk_names() {
    let dml = indoc! { r#"
        datasource test {
          provider = "sqlserver"
          url = "sqlserver://"
        }

        generator js {
          provider = "prisma-client-js"
        }

        model A {
          id Int @id @default(autoincrement())
          a  String @default("asdf", map: "reserved")
        }

        model B {
          id Int @id(map: "reserved") @default(autoincrement())
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The given constraint name `reserved` has to be unique in the following namespace: global for primary keys, foreign keys and default constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0m  id Int @id @default(autoincrement())
        [1;94m12 | [0m  a  String @default("asdf", [1;91mmap: "reserved"[0m)
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@id": The given constraint name `reserved` has to be unique in the following namespace: global for primary keys, foreign keys and default constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0mmodel B {
        [1;94m16 | [0m  id Int @id([1;91mmap: "reserved"[0m) @default(autoincrement())
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn named_default_constraints_cannot_clash_with_fk_names() {
    let dml = indoc! { r#"
        datasource test {
          provider = "sqlserver"
          url = "sqlserver://"
        }

        model A {
          id  Int @id @default(autoincrement())
          a   String  @default("asdf", map: "reserved")
          b   B       @relation(fields: [bId], references: [id], map: "reserved")
          bId Int
        }

        model B {
          id Int @id @default(autoincrement())
          as A[]
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The given constraint name `reserved` has to be unique in the following namespace: global for primary keys, foreign keys and default constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id  Int @id @default(autoincrement())
        [1;94m 8 | [0m  a   String  @default("asdf", [1;91mmap: "reserved"[0m)
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": The given constraint name `reserved` has to be unique in the following namespace: global for primary keys, foreign keys and default constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  a   String  @default("asdf", map: "reserved")
        [1;94m 9 | [0m  b   B       @relation(fields: [bId], references: [id], [1;91mmap: "reserved"[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn default_on_composite_type_field_errors() {
    let schema = indoc! { r#"
        type Address {
            street String
        }

        model User {
            id Int @id
            address Address? @default("{ \"street\": \"broadway\"}")
        }
    "#};

    let error = datamodel::parse_schema(schema).map(drop).unwrap_err();

    // Error could be better since PG don't support composites yet at all.
    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating field `address` in composite type `Address`: Defaults on fields of type composite are not supported. Please remove the `@default` attribute.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m    id Int @id
        [1;94m 7 | [0m    address Address? @[1;91mdefault("{ \"street\": \"broadway\"}")[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error)
}

#[test]
fn must_error_on_auto_default_on_non_native_type_on_mongodb() {
    let schema = r#"
        datasource db {
            provider = "mongodb"
            url = "mongodb://"
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["mongoDb"]
        }

        model User {
            id Int @id @map("_id")
            nickname String @default(auto())
        }
    "#;

    let error = datamodel::parse_schema(schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating field 'nickname': MongoDB `@default(auto())` fields must have `ObjectId` native type and use the `@id` attribute.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m            id Int @id @map("_id")
        [1;94m14 | [0m            [1;91mnickname String @default(auto())[0m
        [1;94m15 | [0m        }
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error)
}

#[test]
fn must_error_on_auto_default_on_non_object_id_on_mongodb() {
    let schema = r#"
        datasource db {
            provider = "mongodb"
            url = "mongodb://"
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["mongoDb"]
        }

        model User {
            id Int @id @map("_id") @default(auto()) @db.Int
            nickname String
        }
    "#;

    let error = datamodel::parse_schema(schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating field 'id': MongoDB `@default(auto())` fields must have `ObjectId` native type and use the `@id` attribute.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m        model User {
        [1;94m13 | [0m            [1;91mid Int @id @map("_id") @default(auto()) @db.Int[0m
        [1;94m14 | [0m            nickname String
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error)
}

#[test]
fn must_error_on_auto_default_on_mongodb_composite() {
    let schema = r#"
        datasource db {
            provider = "mongodb"
            url = "mongodb://"
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["mongoDb"]
        }

        model User {
            id   String @id @map("_id") @default(auto()) @db.ObjectId
            meow Meow
        }

        type Meow {
            id String @default(auto()) @db.ObjectId
        }
    "#;

    let error = datamodel::parse_schema(schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The function `auto()` is not supported on composite fields.[0m
          [1;94m-->[0m  [4mschema.prisma:18[0m
        [1;94m   | [0m
        [1;94m17 | [0m        type Meow {
        [1;94m18 | [0m            id String @[1;91mdefault(auto())[0m @db.ObjectId
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error)
}

#[test]
fn must_error_on_dbgenerated_default_on_mongodb() {
    let schema = r#"
        datasource db {
            provider = "mongodb"
            url = "mongodb://"
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["mongoDb"]
        }

        model User {
            id String @id @map("_id") @default(dbgenerated()) @db.ObjectId
        }
    "#;

    let error = datamodel::parse_schema(schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating field 'id': The `dbgenerated()` function is not allowed with MongoDB. Please use `auto()` instead.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m        model User {
        [1;94m13 | [0m            [1;91mid String @id @map("_id") @default(dbgenerated()) @db.ObjectId[0m
        [1;94m14 | [0m        }
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error)
}

#[test]
fn must_error_with_auto_params_on_mongodb() {
    let schema = r#"
        datasource db {
            provider = "mongodb"
            url = "mongodb://"
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["mongoDb"]
        }

        model User {
            id String @id @map("_id") @default(auto("meow")) @db.ObjectId
        }
    "#;

    let error = datamodel::parse_schema(schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": `auto()` takes no arguments[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m        model User {
        [1;94m13 | [0m            id String @id @map("_id") @[1;91mdefault(auto("meow"))[0m @db.ObjectId
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error)
}

#[test]
fn must_error_if_using_auto_with_postgres() {
    let schema = r#"
        datasource db {
            provider = "postgres"
            url = "postgres://"
        }

        model User {
            id String @id @default(auto())
        }
    "#;

    let error = datamodel::parse_schema(schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The current connector does not support the `auto()` function.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m        model User {
        [1;94m 8 | [0m            id String @id @default([1;91mauto()[0m)
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error)
}

#[test]
fn must_error_if_using_auto_with_mysql() {
    let schema = r#"
        datasource db {
            provider = "mysql"
            url = "mysql://"
        }

        model User {
            id String @id @default(auto())
        }
    "#;

    let error = datamodel::parse_schema(schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The current connector does not support the `auto()` function.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m        model User {
        [1;94m 8 | [0m            id String @id @default([1;91mauto()[0m)
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error)
}

#[test]
fn must_error_if_using_auto_with_sql_server() {
    let schema = r#"
        datasource db {
            provider = "sqlserver"
            url = "sqlserver://"
        }

        model User {
            id String @id @default(auto())
        }
    "#;

    let error = datamodel::parse_schema(schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The current connector does not support the `auto()` function.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m        model User {
        [1;94m 8 | [0m            id String @id @default([1;91mauto()[0m)
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error)
}

#[test]
fn must_error_if_using_auto_with_sqlite() {
    let schema = r#"
        datasource db {
            provider = "sqlite"
            url = "file:dev.db"
        }

        model User {
            id String @id @default(auto())
        }
    "#;

    let error = datamodel::parse_schema(schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The current connector does not support the `auto()` function.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m        model User {
        [1;94m 8 | [0m            id String @id @default([1;91mauto()[0m)
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error)
}

#[test]
fn must_error_on_auto_default_on_non_id_on_mongodb() {
    let schema = r#"
        datasource db {
            provider = "mongodb"
            url = "mongodb://"
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["mongoDb"]
        }

        model User {
            id Int @id @map("_id")
            nickname String @default(auto()) @db.ObjectId
        }
    "#;

    let error = datamodel::parse_schema(schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating field 'nickname': MongoDB `@default(auto())` fields must have `ObjectId` native type and use the `@id` attribute.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m            id Int @id @map("_id")
        [1;94m14 | [0m            [1;91mnickname String @default(auto()) @db.ObjectId[0m
        [1;94m15 | [0m        }
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error)
}

#[test]
fn json_defaults_must_be_valid_json() {
    let schema = r#"
        model Test {
            id Int @id
            name Json @default("not json")
        }
    "#;

    let error = datamodel::parse_schema(schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Parse error: "not json" is not a valid JSON string. (expected ident at line 1 column 2)[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m            id Int @id
        [1;94m 4 | [0m            name Json @default([1;91m"not json"[0m)
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error)
}

#[test]
fn bytes_defaults_must_be_base64() {
    let schema = r#"
        model Test {
            id Int @id
            name Bytes @default("not base64")
        }
    "#;

    let error = datamodel::parse_schema(schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Parse error: "not base64" is not a valid base64 string. (Could not convert from `base64 encoded bytes` to `PrismaValue::Bytes`)[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m            id Int @id
        [1;94m 4 | [0m            name Bytes @default([1;91m"not base64"[0m)
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error)
}

#[test]
fn int_defaults_must_not_contain_decimal_point() {
    let schema = r#"
        model Test {
            id Int @id
            score Int @default(3.14)
        }
    "#;

    let error = datamodel::parse_schema(schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Parse error: "3.14" is not a valid integer. (invalid digit found in string)[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m            id Int @id
        [1;94m 4 | [0m            score Int @default([1;91m3.14[0m)
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error)
}

#[test]
fn bigint_defaults_must_not_contain_decimal_point() {
    let schema = r#"
        model Test {
            id Int @id
            score BigInt @default(3.14)
        }
    "#;

    let error = datamodel::parse_schema(schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Parse error: "3.14" is not a valid integer. (invalid digit found in string)[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m            id Int @id
        [1;94m 4 | [0m            score BigInt @default([1;91m3.14[0m)
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error)
}

#[test]
fn boolean_defaults_must_be_true_or_false() {
    let schema = r#"
        model Test {
            id Int @id
            isEdible Boolean @default(True)
        }
    "#;

    let error = datamodel::parse_schema(schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": A boolean literal must be `true` or `false`.[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m            id Int @id
        [1;94m 4 | [0m            isEdible Boolean @default([1;91mTrue[0m)
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error)
}
