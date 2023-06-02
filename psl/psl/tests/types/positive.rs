use crate::common::*;
use psl::builtin_connectors::{MySqlType, PostgresType};

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

    let datamodel = psl::parse_schema(dml).unwrap();

    let user_model = datamodel.assert_has_model("User");

    let field = user_model.assert_has_scalar_field("name");
    field.assert_default_value().assert_autoincrement();
    field.assert_native_type(datamodel.connector, &PostgresType::SmallInt);
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

    let datamodel = psl::parse_schema(dml).unwrap();
    let user_model = datamodel.assert_has_model("User");

    user_model
        .assert_has_scalar_field("test")
        .assert_native_type(datamodel.connector, &MySqlType::Decimal(Some((8, 2))));
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

    let datamodel = psl::parse_schema(dml).unwrap();
    let user_model = datamodel.assert_has_model("Blog");

    user_model
        .assert_has_scalar_field("foobar")
        .assert_native_type(datamodel.connector, &PostgresType::VarChar(Some(26)));

    user_model
        .assert_has_scalar_field("foobaz")
        .assert_native_type(datamodel.connector, &PostgresType::VarChar(None));
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

    let datamodel = psl::parse_schema(dml).unwrap();
    let user_model = datamodel.assert_has_model("Blog");

    user_model
        .assert_has_scalar_field("smallInt")
        .assert_native_type(datamodel.connector, &MySqlType::SmallInt);

    user_model
        .assert_has_scalar_field("foobar")
        .assert_native_type(datamodel.connector, &MySqlType::DateTime(Some(6)));

    user_model
        .assert_has_scalar_field("fooBool")
        .assert_native_type(datamodel.connector, &MySqlType::TinyInt);

    user_model
        .assert_has_scalar_field("fooInt")
        .assert_native_type(datamodel.connector, &MySqlType::TinyInt);
}
