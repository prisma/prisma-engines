use crate::common::*;
use dml::{DefaultValue, ValueGenerator};
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

    let datamodel = parse(dml);

    let user_model = datamodel.assert_has_model("User");

    user_model
        .assert_has_scalar_field("name")
        .assert_default_value(DefaultValue::new_expression(ValueGenerator::new_autoincrement()));

    let sft = user_model.assert_has_scalar_field("name").assert_native_type();

    let postgres_type: &PostgresType = sft.deserialize_native_type();
    assert_eq!(postgres_type, &PostgresType::SmallInt);
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

    user_model.assert_has_scalar_field("test");

    let sft = user_model.assert_has_scalar_field("test").assert_native_type();

    let mysql_type: &MySqlType = sft.deserialize_native_type();

    assert_eq!(mysql_type, &MySqlType::Decimal(Some((8, 2))));
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
    let postgres_type: &PostgresType = sft.deserialize_native_type();
    assert_eq!(postgres_type, &PostgresType::VarChar(Some(26)));

    let sft = user_model.assert_has_scalar_field("foobaz").assert_native_type();
    let postgres_type: &PostgresType = sft.deserialize_native_type();
    assert_eq!(postgres_type, &PostgresType::VarChar(None));
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

    let mysql_type: &MySqlType = sft.deserialize_native_type();
    assert_eq!(mysql_type, &MySqlType::SmallInt);

    let sft = user_model.assert_has_scalar_field("foobar").assert_native_type();

    let mysql_type: &MySqlType = sft.deserialize_native_type();
    assert_eq!(mysql_type, &MySqlType::DateTime(Some(6)));

    let sft = user_model.assert_has_scalar_field("fooBool").assert_native_type();

    let mysql_type: &MySqlType = sft.deserialize_native_type();
    assert_eq!(mysql_type, &MySqlType::TinyInt);

    let sft = user_model.assert_has_scalar_field("fooInt").assert_native_type();

    let mysql_type: &MySqlType = sft.deserialize_native_type();
    assert_eq!(mysql_type, &MySqlType::TinyInt);
}
