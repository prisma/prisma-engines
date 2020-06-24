use migration_engine_tests::sql::*;
use prisma_value::PrismaValue;
use quaint::prelude::SqlFamily;
use sql_schema_describer::{ColumnTypeFamily, DefaultValue};

#[test_each_connector(capabilities("json"))]
async fn database_level_json_defaults_can_be_defined(api: &TestApi) -> TestResult {
    let dm = format!(
        r#"
            {datasource}

            model Dog {{
                id Int @id
                favouriteThings Json @default("[\"sticks\",\"chimken\",100,  \"dog park\"]")
            }}
        "#,
        datasource = api.datasource()
    );

    api.infer_apply(&dm).send().await?.assert_green()?;

    api.assert_schema().await?.assert_table("Dog", |table| {
        table.assert_column("favouriteThings", |column| {
            column
                .assert_type_family(if api.is_mariadb() {
                    ColumnTypeFamily::String
                } else {
                    ColumnTypeFamily::Json
                })?
                .assert_default(match api.sql_family() {
                    SqlFamily::Postgres => Some(DefaultValue::VALUE(PrismaValue::Json(
                        "[\"sticks\", \"chimken\", 100, \"dog park\"]".into(),
                    ))),
                    SqlFamily::Mysql => None,
                    _ => unreachable!(),
                })
        })
    })?;

    // Check that the migration is idempotent.
    api.infer_apply(&dm).send().await?.assert_green()?.assert_no_steps()?;

    Ok(())
}
