use psl::parser_database::ScalarType;

use crate::common::*;

#[test]
fn should_set_default_for_all_scalar_types() {
    let dml = indoc! {r#"
        datasource db {
          provider = "mongodb"
          url      = "mongodb://"
        }

        type Composite {
          int      Int      @default(3)
          float    Float    @default(3.20)
          string   String   @default("String")
          boolean  Boolean  @default(false)
          dateTime DateTime @default("2019-06-17T14:20:57Z")
          bytes    Bytes    @default("aGVsbG8gd29ybGQ=")
          json     Json     @default("{ \"a\": [\"b\"] }")
        }
    "#};

    let schema = psl::parse_schema(dml).unwrap();
    let composite = schema.assert_has_type("Composite");

    composite
        .assert_has_scalar_field("int")
        .assert_scalar_type(ScalarType::Int)
        .assert_default_value()
        .assert_int(3);

    composite
        .assert_has_scalar_field("float")
        .assert_scalar_type(ScalarType::Float);

    composite
        .assert_has_scalar_field("string")
        .assert_scalar_type(ScalarType::String)
        .assert_default_value()
        .assert_string("String");

    composite
        .assert_has_scalar_field("boolean")
        .assert_scalar_type(ScalarType::Boolean)
        .assert_default_value()
        .assert_bool(false);

    composite
        .assert_has_scalar_field("dateTime")
        .assert_scalar_type(ScalarType::DateTime);

    composite
        .assert_has_scalar_field("bytes")
        .assert_scalar_type(ScalarType::Bytes)
        .assert_default_value()
        .assert_bytes(b"hello world");

    composite
        .assert_has_scalar_field("json")
        .assert_scalar_type(ScalarType::Json)
        .assert_default_value()
        .assert_string(r#"{ "a": ["b"] }"#);
}

#[test]
fn should_set_default_an_enum_type() {
    let dml = indoc! {r#"
        datasource db {
          provider = "mongodb"
          url      = "mongodb://"
        }

        type Composite {
          id   Int
          role Role @default(A_VARIANT_WITH_UNDERSCORES)
        }

        enum Role {
          ADMIN
          MODERATOR
          A_VARIANT_WITH_UNDERSCORES
        }
    "#};

    psl::parse_schema(dml)
        .unwrap()
        .assert_has_type("Composite")
        .assert_has_scalar_field("role")
        .assert_default_value()
        .assert_constant("A_VARIANT_WITH_UNDERSCORES");
}

#[test]
fn should_set_default_on_remapped_enum_type() {
    let dml = indoc! {r#"
        datasource db {
          provider = "mongodb"
          url      = "mongodb://"
        }

        type Composite {
          id   Int
          role Role @default(A_VARIANT_WITH_UNDERSCORES)
        }

        enum Role {
          ADMIN
          MODERATOR
          A_VARIANT_WITH_UNDERSCORES @map("A VARIANT WITH UNDERSCORES")
        }
    "#};

    psl::parse_schema(dml)
        .unwrap()
        .assert_has_type("Composite")
        .assert_has_scalar_field("role")
        .assert_default_value()
        .assert_constant("A_VARIANT_WITH_UNDERSCORES");
}

#[test]
fn string_literals_with_double_quotes_work() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mongodb"
          url      = "mongodb://"
        }

        type Test {
          id    String @default("abcd")
          name  String @default("ab\"c\"d")
          name2 String @default("\"")
        }
    "#};

    let schema = psl::parse_schema(schema).unwrap();
    let composite = schema.assert_has_type("Test");

    composite
        .assert_has_scalar_field("id")
        .assert_scalar_type(ScalarType::String)
        .assert_default_value()
        .assert_string("abcd");

    composite
        .assert_has_scalar_field("name")
        .assert_scalar_type(ScalarType::String)
        .assert_default_value()
        .assert_string("ab\"c\"d");

    composite
        .assert_has_scalar_field("name2")
        .assert_scalar_type(ScalarType::String)
        .assert_default_value()
        .assert_string("\"");
}
