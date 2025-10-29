use prisma_value::PrismaValue;
use sql_migration_tests::test_api::*;
use sql_schema_describer::{ColumnTypeFamily, DefaultValue};

#[test_connector(capabilities(Json))]
fn json_fields_can_be_created(api: TestApi) {
    let dm = r#"
            model Test {
                id String @id @default(cuid())
                javaScriptObjectNotation Json
            }
        "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("Test", |table| {
        table.assert_column("javaScriptObjectNotation", |c| {
            if api.is_mariadb() {
                // JSON is an alias for LONGTEXT on MariaDB - https://mariadb.com/kb/en/json-data-type/
                c.assert_is_required().assert_type_family(ColumnTypeFamily::String)
            } else {
                c.assert_is_required().assert_type_family(ColumnTypeFamily::Json)
            }
        })
    });

    api.schema_push_w_datasource(dm).send().assert_green().assert_no_steps();
}

#[test_connector(capabilities(Json))]
fn database_level_json_defaults_can_be_defined(api: TestApi) {
    let dm = r#"
            model Dog {
                id Int @id
                favouriteThings Json @default("[\"sticks\",\"chimken\",100,  \"dog park\"]")
            }
        "#;

    api.schema_push_w_datasource(dm).send().assert_green();

    api.assert_schema().assert_table("Dog", |table| {
        table.assert_column("favouriteThings", |column| {
            column
                .assert_type_family(if api.is_mariadb() {
                    ColumnTypeFamily::String
                } else {
                    ColumnTypeFamily::Json
                })
                .assert_default(if api.is_postgres() {
                    Some(DefaultValue::value(PrismaValue::String(
                        "[\"sticks\", \"chimken\", 100, \"dog park\"]".into(),
                    )))
                } else if api.is_mysql() {
                    None
                } else if api.is_sqlite() {
                    Some(DefaultValue::value(PrismaValue::String(
                        "[\"sticks\",\"chimken\",100,  \"dog park\"]".into(),
                    )))
                } else {
                    unreachable!()
                })
        })
    });

    // Check that the migration is idempotent.
    api.schema_push_w_datasource(dm).send().assert_green().assert_no_steps();
}
