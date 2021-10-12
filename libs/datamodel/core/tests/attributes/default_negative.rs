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
        [1;91merror[0m: [1mError parsing attribute "@default": Expected a String value, but received numeric value `3`.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  rel String @[1;91mdefault(3)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_if_default_value_parser_error() {
    let dml = indoc! {r#"
        model Model {
          id Int @id
          rel DateTime @default("Hugo")
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Expected a datetime value, but failed while parsing ""Hugo"": input contains invalid characters.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  rel DateTime @[1;91mdefault("Hugo")[0m
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
        [1;91merror[0m: [1mError parsing attribute "@default": The function unknown_function is not a known function.[0m
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
        [1;91merror[0m: [1mError parsing attribute "@default": The defined default value is not a valid value of the enum specified for the field.[0m
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
        [1;91merror[0m: [1mError parsing attribute "@default": Expected a function value, but received numeric value `12`.[0m
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
        [1;91merror[0m: [1mError parsing attribute "@default": Error validating: DefaultValue function parsing failed. The function arg should only be empty or a single String. Got: `cuid()`. You can read about the available functions here: https://pris.ly/d/attribute-functions[0m
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
        [1;91merror[0m: [1mError parsing attribute "@default": Error validating: DefaultValue function parsing failed. The function arg should only be empty or a single String. Got: `5`. You can read about the available functions here: https://pris.ly/d/attribute-functions[0m
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
          previewFeatures = ["namedConstraints"]
        }

        model A {
          id Int @id @default(autoincrement())
          data String @default("beeb buub", map: "meow")
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": You defined a database name for the default value of a field on the model. This is not supported by the provider.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id Int @id @default(autoincrement())
        [1;94m13 | [0m  data String @[1;91mdefault("beeb buub", map: "meow")[0m
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
          previewFeatures = ["namedConstraints"]
        }

        model A {
          id Int @id @default(autoincrement(), map: "nope__nope__nope")
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Naming an autoincrement default value is not allowed.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mmodel A {
        [1;94m12 | [0m  id Int @id @[1;91mdefault(autoincrement(), map: "nope__nope__nope")[0m
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
          previewFeatures = ["namedConstraints"]
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
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id Int @id @default(autoincrement())
        [1;94m13 | [0m  a  String @default("asdf", [1;91mmap: "reserved"[0m)
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@default": The given constraint name `reserved` has to be unique in the following namespace: global for primary keys, foreign keys and default constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:18[0m
        [1;94m   | [0m
        [1;94m17 | [0m  id Int @id @default(autoincrement())
        [1;94m18 | [0m  b  String @default("asdf", [1;91mmap: "reserved"[0m)
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
          previewFeatures = ["namedConstraints"]
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
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id Int @id @default(autoincrement())
        [1;94m13 | [0m  a  String @default("asdf", [1;91mmap: "reserved"[0m)
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@id": The given constraint name `reserved` has to be unique in the following namespace: global for primary keys, foreign keys and default constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0mmodel B {
        [1;94m17 | [0m  id Int @[1;91mid(map: "reserved")[0m @default(autoincrement())
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
        datasource db {
            provider = "mongodb"
            url = "mongodb://"
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["mongoDb"]
        }

        type Address {
            street String
        }

        model User {
            id Int @id
            address Address? @default("{ \"street\": \"broadway\"}")
        }
    "#};

    let error = datamodel::parse_schema(schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating field `address` in composite type `Address`: Defaults inside composite types are not supported[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m    id Int @id
        [1;94m17 | [0m    address Address? @[1;91mdefault("{ \"street\": \"broadway\"}")[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error)
}

#[test]
fn default_inside_composite_type_field_errors() {
    let schema = indoc! { r#"
        datasource db {
            provider = "mongodb"
            url = "mongodb://"
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["mongoDb"]
        }

        type Address {
            street String @default("Champs Elysees")
        }

        model User {
            id Int @id
            address Address?
        }
    "#};

    let error = datamodel::parse_schema(schema).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mAttribute not known: "@default".[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mtype Address {
        [1;94m12 | [0m    street String @[1;91mdefault[0m("Champs Elysees")
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error)
}
