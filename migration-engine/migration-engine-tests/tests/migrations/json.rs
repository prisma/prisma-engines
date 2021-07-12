use migration_engine_tests::sync_test_api::*;
use prisma_value::PrismaValue;
use sql_schema_describer::{ColumnTypeFamily, DefaultValue};

#[test_connector(capabilities(Json), exclude(Mysql56))]
fn json_fields_can_be_created(api: TestApi) {
    let dm = format!(
        r#"
            {}

            model Test {{
                id String @id @default(cuid())
                javaScriptObjectNotation Json
            }}
        "#,
        api.datasource_block()
    );

    api.schema_push(&dm).send().assert_green_bang();

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

    api.schema_push(&dm).send().assert_green_bang().assert_no_steps();
}

#[test_connector(capabilities(Json), exclude(Mysql56))]
fn database_level_json_defaults_can_be_defined(api: TestApi) {
    let dm = format!(
        r#"
            {datasource}

            model Dog {{
                id Int @id
                favouriteThings Json @default("[\"sticks\",\"chimken\",100,  \"dog park\"]")
            }}
        "#,
        datasource = api.datasource_block()
    );

    api.schema_push(&dm).send().assert_green_bang();

    api.assert_schema().assert_table("Dog", |table| {
        table.assert_column("favouriteThings", |column| {
            column
                .assert_type_family(if api.is_mariadb() {
                    ColumnTypeFamily::String
                } else {
                    ColumnTypeFamily::Json
                })
                .assert_default(if api.is_postgres() {
                    Some(DefaultValue::value(PrismaValue::Json(
                        "[\"sticks\", \"chimken\", 100, \"dog park\"]".into(),
                    )))
                } else if api.is_mysql() {
                    None
                } else {
                    unreachable!()
                })
        })
    });

    // Check that the migration is idempotent.
    api.schema_push(&dm).send().assert_green_bang().assert_no_steps();
}
