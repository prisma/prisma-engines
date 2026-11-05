use quaint::Value;
use sql_migration_tests::test_api::*;

#[test_connector(tags(KingbaseOracle))]
fn oracle_varchar2_widening_preserves_existing_data(api: TestApi) {
    let initial_schema = r#"
        model Account {
            id   Int    @id
            code String @db.VarChar2(32)
        }
    "#;

    api.schema_push_w_datasource(initial_schema).send().assert_green();
    api.insert("Account")
        .value("id", 1)
        .value("code", "kingbase")
        .result_raw();

    let widened_schema = r#"
        model Account {
            id   Int    @id
            code String @db.VarChar2(64)
        }
    "#;

    api.schema_push_w_datasource(widened_schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.dump_table("Account")
        .assert_single_row(|row| row.assert_text_value("code", "kingbase"));
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_number_precision_widening_preserves_existing_data(api: TestApi) {
    let initial_schema = r#"
        model Ledger {
            id     Int     @id
            amount Decimal @db.Number(10, 2)
        }
    "#;

    api.schema_push_w_datasource(initial_schema).send().assert_green();
    api.insert("Ledger")
        .value("id", 1)
        .value("amount", Value::numeric("12.34".parse().unwrap()))
        .result_raw();

    let widened_schema = r#"
        model Ledger {
            id     Int     @id
            amount Decimal @db.Number(12, 2)
        }
    "#;

    api.schema_push_w_datasource(widened_schema)
        .send()
        .assert_green()
        .assert_has_executed_steps();

    api.assert_schema().assert_table("Ledger", |table| {
        table.assert_column("amount", |column| {
            column.assert_native_type("Number(12,2)", psl::builtin_connectors::KINGBASE_ORACLE)
        })
    });
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_blob_to_number_with_existing_data_is_reported_as_data_loss(api: TestApi) {
    let initial_schema = r#"
        model Payload {
            id   Int   @id
            data Bytes @db.Blob
        }
    "#;

    api.schema_push_w_datasource(initial_schema).send().assert_green();
    api.insert("Payload")
        .value("id", 1)
        .value("data", Value::bytes(vec![0x01, 0x02]))
        .result_raw();

    let changed_schema = r#"
        model Payload {
            id   Int @id
            data Int
        }
    "#;

    api.schema_push_w_datasource(changed_schema)
        .send()
        .assert_no_warning()
        .assert_unexecutable(&[
            "Changed the type of `data` on the `Payload` table. No cast exists, the column would be dropped and recreated, which cannot be done since the column is required and there is data in the table."
                .into(),
        ])
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_making_a_populated_optional_column_required_is_unexecutable(api: TestApi) {
    let initial_schema = r#"
        model Account {
            id   Int     @id
            code String? @db.VarChar2(32)
        }
    "#;

    api.schema_push_w_datasource(initial_schema).send().assert_green();
    api.insert("Account").value("id", 1).result_raw();

    let changed_schema = r#"
        model Account {
            id   Int    @id
            code String @db.VarChar2(32)
        }
    "#;

    api.schema_push_w_datasource(changed_schema)
        .send()
        .assert_unexecutable(&[
            "Made the column `code` on table `Account` required, but there are 1 existing NULL values.".into(),
        ])
        .assert_no_steps();
}

#[test_connector(tags(KingbaseOracle))]
fn oracle_varchar2_narrowing_with_fitting_data_executes_with_a_warning(api: TestApi) {
    let initial_schema = r#"
        model Account {
            id   Int    @id
            code String @db.VarChar2(64)
        }
    "#;

    api.schema_push_w_datasource(initial_schema).send().assert_green();
    api.insert("Account").value("id", 1).value("code", "short").result_raw();

    let changed_schema = r#"
        model Account {
            id   Int    @id
            code String @db.VarChar2(32)
        }
    "#;

    api.schema_push_w_datasource(changed_schema)
        .force(true)
        .send()
        .assert_executable()
        .assert_has_executed_steps();

    api.dump_table("Account")
        .assert_single_row(|row| row.assert_text_value("code", "short"));
}
