use crate::common::*;

#[test]
fn should_set_default_for_all_scalar_types() {
    let dml = r#"
    datasource db {
        provider = "mongodb"
        url = "mongodb://"
    }

    type Composite {
        int Int @default(3)
        float Float @default(3.20)
        string String @default("String")
        boolean Boolean @default(false)
        dateTime DateTime @default("2019-06-17T14:20:57Z")
        bytes    Bytes @default("aGVsbG8gd29ybGQ=")
        json     Json  @default("{ \"a\": [\"b\"] }")
    }
    "#;

    let datamodel = parse(dml);
    let user_composite = datamodel.assert_has_composite_type("Composite");
    user_composite
        .assert_has_scalar_field("int")
        .assert_base_type(&ScalarType::Int)
        .assert_default_value(DefaultValue::new_single(PrismaValue::Int(3)));
    user_composite
        .assert_has_scalar_field("float")
        .assert_base_type(&ScalarType::Float);

    user_composite
        .assert_has_scalar_field("string")
        .assert_base_type(&ScalarType::String)
        .assert_default_value(DefaultValue::new_single(PrismaValue::String(String::from("String"))));
    user_composite
        .assert_has_scalar_field("boolean")
        .assert_base_type(&ScalarType::Boolean)
        .assert_default_value(DefaultValue::new_single(PrismaValue::Boolean(false)));
    user_composite
        .assert_has_scalar_field("dateTime")
        .assert_base_type(&ScalarType::DateTime);

    user_composite
        .assert_has_scalar_field("bytes")
        .assert_base_type(&ScalarType::Bytes)
        .assert_default_value(DefaultValue::new_single(PrismaValue::Bytes(b"hello world".to_vec())));
    user_composite
        .assert_has_scalar_field("json")
        .assert_base_type(&ScalarType::Json)
        .assert_default_value(DefaultValue::new_single(PrismaValue::Json(
            r#"{ "a": ["b"] }"#.to_owned(),
        )));
}

#[test]
fn should_set_default_an_enum_type() {
    let dml = r#"
    datasource db {
        provider = "mongodb"
        url = "mongodb://"
    }

    type Composite {
        id Int
        role Role @default(A_VARIANT_WITH_UNDERSCORES)
    }

    enum Role {
        ADMIN
        MODERATOR
        A_VARIANT_WITH_UNDERSCORES
    }
    "#;

    let datamodel = parse(dml);
    let user_composite = datamodel.assert_has_composite_type("Composite");
    user_composite
        .assert_has_enum_field("role")
        .assert_enum_type("Role")
        .assert_default_value(DefaultValue::new_single(PrismaValue::Enum(String::from(
            "A_VARIANT_WITH_UNDERSCORES",
        ))));
}

// TODO: Remove ignore when enums are supported
#[test]
fn should_set_default_on_remapped_enum_type() {
    let dml = r#"
    datasource db {
        provider = "mongodb"
        url = "mongodb://"
    }

    type Composite {
        id Int
        role Role @default(A_VARIANT_WITH_UNDERSCORES)
    }

    enum Role {
        ADMIN
        MODERATOR
        A_VARIANT_WITH_UNDERSCORES @map("A VARIANT WITH UNDERSCORES")
    }
    "#;

    let datamodel = parse(dml);
    let user_composite = datamodel.assert_has_composite_type("Composite");
    user_composite
        .assert_has_enum_field("role")
        .assert_enum_type("Role")
        .assert_default_value(DefaultValue::new_single(PrismaValue::Enum(String::from(
            "A_VARIANT_WITH_UNDERSCORES",
        ))));
}

#[test]
fn string_literals_with_double_quotes_work() {
    let schema = r#"
        datasource db {
            provider = "mongodb"
            url = "mongodb://"
        }

        type Test {
            id   String @default("abcd")
            name String @default("ab\"c\"d")
            name2 String @default("\"")
        }
    "#;

    let datamodel = parse(schema);
    let test_composite = datamodel.assert_has_composite_type("Test");
    test_composite
        .assert_has_scalar_field("id")
        .assert_base_type(&ScalarType::String)
        .assert_default_value(DefaultValue::new_single(PrismaValue::String(String::from("abcd"))));
    test_composite
        .assert_has_scalar_field("name")
        .assert_base_type(&ScalarType::String)
        .assert_default_value(DefaultValue::new_single(PrismaValue::String(String::from("ab\"c\"d"))));
    test_composite
        .assert_has_scalar_field("name2")
        .assert_base_type(&ScalarType::String)
        .assert_default_value(DefaultValue::new_single(PrismaValue::String(String::from("\""))));
}
