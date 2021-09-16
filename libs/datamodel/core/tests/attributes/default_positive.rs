use crate::common::*;
use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::DateTime;
use datamodel::{DefaultValue, ScalarType, ValueGenerator};
use indoc::indoc;
use prisma_value::PrismaValue;

#[test]
fn should_set_default_for_all_scalar_types() {
    let dml = r#"
    model Model {
        id Int @id
        int Int @default(3)
        float Float @default(3.20)
        string String @default("String")
        boolean Boolean @default(false)
        dateTime DateTime @default("2019-06-17T14:20:57Z")
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
            previewFeatures = ["namedConstraints"]
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

// TODO: Change me when we do validate
#[test]
fn named_default_constraints_should_not_validate_name_clashes_on_pk() {
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
            id Int @id(map: "meow") @default(autoincrement())
        }

        model B {
            id Int @id(map: "meow") @default(autoincrement())
        }
    "#};

    assert!(datamodel::parse_schema(dml).is_ok());
}

// TODO: Change me when we do validate
#[test]
fn named_default_constraints_should_not_validate_name_clashes_on_pk_fk() {
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
            id  Int @id(map: "meow") @default(autoincrement())
            b   B   @relation(fields: [bId], references: [id], map: "meow")
            bId Int
        }

        model B {
            id Int @id(map: "meow") @default(autoincrement())
            as A[]
        }
    "#};

    assert!(datamodel::parse_schema(dml).is_ok());
}

// TODO: Change me when we do validate
#[test]
fn named_default_constraints_should_not_validate_name_clashes_on_fk() {
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
            id  Int @id(map: "meow") @default(autoincrement())
            b   B   @relation(fields: [bId], references: [id], map: "meow")
            c   C   @relation(fields: [cId], references: [id], map: "meow")
            bId Int
            cId Int
        }

        model B {
            id Int @id @default(autoincrement())
            as A[]
        }

        model C {
            id Int @id @default(autoincrement())
            as A[]
        }
    "#};

    assert!(datamodel::parse_schema(dml).is_ok());
}
