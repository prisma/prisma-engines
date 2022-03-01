use crate::common::*;
use bigdecimal::{BigDecimal, FromPrimitive};
use datamodel::{dml::ScalarType, DefaultValue, ValueGenerator};
use dml::prisma_value::PrismaValue;
use native_types::{MySqlType, PostgresType};

#[test]
fn should_apply_a_custom_type() {
    let dml = r#"
    type ID = String @id @default(cuid())

    model Model {
        id ID
    }
    "#;

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_scalar_field("id")
        .assert_is_id(user_model)
        .assert_base_type(&ScalarType::String)
        .assert_default_value(DefaultValue::new_expression(ValueGenerator::new_cuid()));
}

#[test]
fn should_recursively_apply_a_custom_type() {
    let dml = r#"
        type MyString = String
        type MyStringWithDefault = MyString @default(cuid())
        type ID = MyStringWithDefault @id

        model Model {
            id ID
        }
    "#;

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("Model");
    user_model
        .assert_has_scalar_field("id")
        .assert_is_id(user_model)
        .assert_base_type(&ScalarType::String)
        .assert_default_value(DefaultValue::new_expression(ValueGenerator::new_cuid()));
}

#[test]
fn should_be_able_to_handle_native_type_combined_with_default_autoincrement_attribute() {
    let dml = r#"
        datasource db {
            provider        = "postgres"
            url             = "postgresql://"
        }

        model User {
            id   Int @id
            name Int    @default(autoincrement()) @db.SmallInt
        }

    "#;

    let datamodel = parse(dml);

    let user_model = datamodel.assert_has_model("User");

    user_model
        .assert_has_scalar_field("name")
        .assert_default_value(DefaultValue::new_expression(ValueGenerator::new_autoincrement()));

    let sft = user_model.assert_has_scalar_field("name").assert_native_type();

    let postgres_type: PostgresType = sft.deserialize_native_type();
    assert_eq!(postgres_type, PostgresType::SmallInt);
}

#[test]
fn should_be_able_to_handle_native_type_combined_with_default_attribute() {
    let dml = r#"
        datasource db {
            provider        = "mysql"
            url             = "mysql://"
        }

        model User {
            id    Int      @id
            test  Decimal  @default(1.00) @db.Decimal(8, 2)
        }
    "#;

    let datamodel = parse(dml);

    let user_model = datamodel.assert_has_model("User");

    user_model
        .assert_has_scalar_field("test")
        .assert_default_value(DefaultValue::new_single(PrismaValue::Float(
            BigDecimal::from_f64(1.00).unwrap(),
        )));

    let sft = user_model.assert_has_scalar_field("test").assert_native_type();

    let mysql_type: MySqlType = sft.deserialize_native_type();

    assert_eq!(mysql_type, MySqlType::Decimal(Some((8, 2))));
}

#[test]
fn should_be_able_to_handle_multiple_types() {
    let dml = r#"
    type ID = String @id @default(cuid())
    type UniqueString = String @unique
    type Cash = Int @default(0)

    model User {
        id       ID
        email    UniqueString
        balance  Cash
    }
    "#;

    let datamodel = parse(dml);
    let user_model = datamodel.assert_has_model("User");
    user_model
        .assert_has_scalar_field("id")
        .assert_is_id(user_model)
        .assert_base_type(&ScalarType::String)
        .assert_default_value(DefaultValue::new_expression(ValueGenerator::new_cuid()));

    user_model
        .assert_has_scalar_field("email")
        .assert_base_type(&ScalarType::String);

    assert!(user_model.field_is_unique("email"));

    user_model
        .assert_has_scalar_field("balance")
        .assert_base_type(&ScalarType::Int)
        .assert_default_value(DefaultValue::new_single(PrismaValue::Int(0)));
}

#[test]
fn should_be_able_to_define_custom_enum_types() {
    let dml = r#"
    type RoleWithDefault = Role @default(USER)

    model User {
        id Int @id
        role RoleWithDefault
    }

    enum Role {
        ADMIN
        USER
        CEO
    }
    "#;

    let datamodel = parse(dml);

    let user_model = datamodel.assert_has_model("User");

    user_model
        .assert_has_scalar_field("role")
        .assert_enum_type("Role")
        .assert_default_value(DefaultValue::new_single(PrismaValue::Enum(String::from("USER"))));
}

#[test]
fn should_handle_type_specifications_on_postgres() {
    let dml = r#"
        datasource pg {
          provider = "postgres"
          url = "postgresql://"
        }

        model Blog {
            id     Int    @id
            foobar String @pg.VarChar(26)
            foobaz String @pg.VarChar
        }
    "#;

    let datamodel = parse(dml);

    let user_model = datamodel.assert_has_model("Blog");

    let sft = user_model.assert_has_scalar_field("foobar").assert_native_type();
    let postgres_type: PostgresType = sft.deserialize_native_type();
    assert_eq!(postgres_type, PostgresType::VarChar(Some(26)));

    let sft = user_model.assert_has_scalar_field("foobaz").assert_native_type();
    let postgres_type: PostgresType = sft.deserialize_native_type();
    assert_eq!(postgres_type, PostgresType::VarChar(None));
}

#[test]
fn should_handle_type_specifications_on_mysql() {
    let dml = r#"
        datasource mys {
          provider = "mysql"
          url = "mysql://"
        }

        model Blog {
            id       Int      @id
            smallInt Int      @mys.SmallInt
            foobar   DateTime @mys.DateTime(6)
            fooBool  Boolean  @mys.TinyInt
            fooInt   Int      @mys.TinyInt
        }
    "#;

    let datamodel = parse(dml);

    let user_model = datamodel.assert_has_model("Blog");

    let sft = user_model.assert_has_scalar_field("smallInt").assert_native_type();

    let mysql_type: MySqlType = sft.deserialize_native_type();
    assert_eq!(mysql_type, MySqlType::SmallInt);

    let sft = user_model.assert_has_scalar_field("foobar").assert_native_type();

    let mysql_type: MySqlType = sft.deserialize_native_type();
    assert_eq!(mysql_type, MySqlType::DateTime(Some(6)));

    let sft = user_model.assert_has_scalar_field("fooBool").assert_native_type();

    let mysql_type: MySqlType = sft.deserialize_native_type();
    assert_eq!(mysql_type, MySqlType::TinyInt);

    let sft = user_model.assert_has_scalar_field("fooInt").assert_native_type();

    let mysql_type: MySqlType = sft.deserialize_native_type();
    assert_eq!(mysql_type, MySqlType::TinyInt);
}
