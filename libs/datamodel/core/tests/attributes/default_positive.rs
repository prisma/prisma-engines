use crate::common::*;
use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::DateTime;

#[test]
fn should_set_default_for_all_scalar_types() {
    let dml = r#"
    datasource test {
        provider = "postgresql"
        url = "postgresql://"
    }

    generator js {
        provider = "prisma-client-js"
    }

    model Model {
        id Int @id
        int Int @default(3)
        float Float @default(3.20)
        string String @default("String")
        boolean Boolean @default(false)
        dateTime DateTime @default("2019-06-17T14:20:57Z")
        bytes    Bytes @default("aGVsbG8gd29ybGQ=")
        json     Json  @default("{ \"a\": [\"b\"] }")
        decimal  Decimal  @default("121.10299000124800000001")
    }
    "#;

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_scalar_field("int")
        .assert_base_type(&ScalarType::Int)
        .assert_default_value(DefaultValue::new_single(PrismaValue::Int(3)));
    user_model
        .assert_has_scalar_field("float")
        .assert_base_type(&ScalarType::Float)
        .assert_default_value(DefaultValue::new_single(PrismaValue::Float(
            BigDecimal::from_f64(3.20).unwrap(),
        )));

    user_model
        .assert_has_scalar_field("string")
        .assert_base_type(&ScalarType::String)
        .assert_default_value(DefaultValue::new_single(PrismaValue::String(String::from("String"))));
    user_model
        .assert_has_scalar_field("boolean")
        .assert_base_type(&ScalarType::Boolean)
        .assert_default_value(DefaultValue::new_single(PrismaValue::Boolean(false)));
    user_model
        .assert_has_scalar_field("dateTime")
        .assert_base_type(&ScalarType::DateTime)
        .assert_default_value(DefaultValue::new_single(PrismaValue::DateTime(
            DateTime::parse_from_rfc3339("2019-06-17T14:20:57Z").unwrap(),
        )));
    user_model
        .assert_has_scalar_field("bytes")
        .assert_base_type(&ScalarType::Bytes)
        .assert_default_value(DefaultValue::new_single(PrismaValue::Bytes(b"hello world".to_vec())));
    user_model
        .assert_has_scalar_field("json")
        .assert_base_type(&ScalarType::Json)
        .assert_default_value(DefaultValue::new_single(PrismaValue::Json(
            r#"{ "a": ["b"] }"#.to_owned(),
        )));
    user_model
        .assert_has_scalar_field("decimal")
        .assert_base_type(&ScalarType::Decimal)
        .assert_default_value(DefaultValue::new_single(PrismaValue::Float(
            r#"121.10299000124800000001"#.parse().unwrap(),
        )));
}

#[test]
fn should_set_default_an_enum_type() {
    let dml = r#"
    model Model {
        id Int @id
        role Role @default(A_VARIANT_WITH_UNDERSCORES)
    }

    enum Role {
        ADMIN
        MODERATOR
        A_VARIANT_WITH_UNDERSCORES
    }
    "#;

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_scalar_field("role")
        .assert_enum_type("Role")
        .assert_default_value(DefaultValue::new_single(PrismaValue::Enum(String::from(
            "A_VARIANT_WITH_UNDERSCORES",
        ))));
}

#[test]
fn should_set_default_on_remapped_enum_type() {
    let dml = r#"
    model Model {
        id Int @id
        role Role @default(A_VARIANT_WITH_UNDERSCORES)
    }

    enum Role {
        ADMIN
        MODERATOR
        A_VARIANT_WITH_UNDERSCORES @map("A VARIANT WITH UNDERSCORES")
    }
    "#;

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_scalar_field("role")
        .assert_enum_type("Role")
        .assert_default_value(DefaultValue::new_single(PrismaValue::Enum(String::from(
            "A_VARIANT_WITH_UNDERSCORES",
        ))));
}

#[test]
fn db_generated_function_must_work_for_enum_fields() {
    let dml = r#"
    model Model {
        id Int @id
        role Role @default(dbgenerated("ADMIN"))
    }

    enum Role {
        ADMIN
        MODERATOR
    }
    "#;

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");

    user_model
        .assert_has_scalar_field("role")
        .assert_enum_type("Role")
        .assert_default_value(DefaultValue::new_expression(ValueGenerator::new_dbgenerated(
            "ADMIN".to_string(),
        )));
}

#[test]
fn named_default_constraints_should_work_on_sql_server() {
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
            data String @default("beeb buub", map: "meow")
        }
    "#};

    let mut expected_default = DefaultValue::new_single(PrismaValue::String(String::from("beeb buub")));
    expected_default.set_db_name("meow");

    parse(dml)
        .assert_has_model("A")
        .assert_has_scalar_field("data")
        .assert_default_value(expected_default);
}

#[test]
fn string_literals_with_double_quotes_work() {
    let schema = r#"
        model Test {
            id   String @id @default("abcd")
            name String @default("ab\"c\"d")
            name2 String @default("\"")
        }
    "#;

    let (_, datamodel) = datamodel::parse_schema(schema).unwrap();
    let test_model = datamodel.assert_has_model("Test");
    test_model
        .assert_has_scalar_field("id")
        .assert_base_type(&ScalarType::String)
        .assert_default_value(DefaultValue::new_single(PrismaValue::String(String::from("abcd"))));
    test_model
        .assert_has_scalar_field("name")
        .assert_base_type(&ScalarType::String)
        .assert_default_value(DefaultValue::new_single(PrismaValue::String(String::from("ab\"c\"d"))));
    test_model
        .assert_has_scalar_field("name2")
        .assert_base_type(&ScalarType::String)
        .assert_default_value(DefaultValue::new_single(PrismaValue::String(String::from("\""))));
}

#[test]
fn mongodb_auto_id() {
    let dml = indoc! {r#"
        datasource db {
          provider = "mongodb"
          url = env("DATABASE_URL")
        }

        model a {
          id String @id @default(auto()) @db.ObjectId @map("_id")
        }
    "#};

    let (_, datamodel) = datamodel::parse_schema(dml).unwrap();

    datamodel
        .assert_has_model("a")
        .assert_has_scalar_field("id")
        .assert_default_value(DefaultValue::new_expression(ValueGenerator::new_auto()));
}

#[test]
fn scalar_list_defaults_with_decimal() {
    let dml = indoc! {r#"
        datasource db {
          provider = "postgresql"
          url = "postgres://"
        }

        enum Color {
            RED
            GREEN
            BLUE
        }

        model Model {
            id Int @id
            int_empty Int[] @default([])
            int Int[] @default([0, 1, 1, 2, 3, 5, 8, 13, 21])
            float Float[] @default([3.20, 4.20, 3.14, 0, 9.9999999, 1000.7])
            string String[] @default(["Arrabiata", "Carbonara", "Al Ragù"])
            boolean Boolean[] @default([false, true ,true, true])
            dateTime DateTime[] @default(["2019-06-17T14:20:57Z", "2020-09-21T20:00:00+02:00"])
            colors Color[] @default([GREEN, BLUE])
            colors_empty Color[] @default([])
            bytes    Bytes[] @default(["aGVsbG8gd29ybGQ="])
            json     Json[]  @default(["{ \"a\": [\"b\"] }", "3"])
            decimal  Decimal[]  @default(["121.10299000124800000001", "0.4", "1.1", "-68.0"])
        }
    "#};

    assert_valid(dml);
}

#[test]
fn scalar_list_defaults_with_composite_types() {
    let dml = indoc! {r#"
        datasource db {
          provider = "mongodb"
          url = "mongodb://"
        }

        enum Color {
            RED
            GREEN
            BLUE
        }

        model Model {
            id Int @id @map("_id")
            int_empty Int[] @default([])
            int Int[] @default([0, 1, 1, 2, 3, 5, 8, 13, 21])
            float Float[] @default([3.20, 4.20, 3.14, 0, 9.9999999, 1000.7])
            string String[] @default(["Arrabiata", "Carbonara", "Al Ragù"])
            boolean Boolean[] @default([false, true ,true, true])
            dateTime DateTime[] @default(["2019-06-17T14:20:57Z", "2020-09-21T20:00:00+02:00"])
            colors Color[] @default([GREEN, BLUE])
            colors_empty Color[] @default([])
            bytes    Bytes[] @default(["aGVsbG8gd29ybGQ="])
            json     Json[]  @default(["{ \"a\": [\"b\"] }", "3"])
        }

        type CompositeT {
            int_empty Int[] @default([])
            int Int[] @default([0, 1, 1, 2, 3, 5, 8, 13, 21])
            float Float[] @default([3.20, 4.20, 3.14, 0, 9.9999999, 1000.7])
            string String[] @default(["Arrabiata", "Carbonara", "Al Ragù"])
            boolean Boolean[] @default([false, true ,true, true])
            dateTime DateTime[] @default(["2019-06-17T14:20:57Z", "2020-09-21T20:00:00+02:00"])
            bytes    Bytes[] @default(["aGVsbG8gd29ybGQ="])
            colors Color[] @default([GREEN, BLUE])
            colors_empty Color[] @default([])
            json     Json[]  @default(["{ \"a\": [\"b\"] }", "3"])
        }
    "#};

    assert_valid(dml);
}
