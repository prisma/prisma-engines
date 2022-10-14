use query_engine_tests::*;

#[test_suite(only(Postgres))]
mod param_type_changes {
    fn schema() -> String {
        r#"
            model TestModel {
                id  Int @id @default(autoincrement())
                col String? @test.Uuid
            }
        "#
        .to_owned()
    }

    // MULTI TRACK DRIFTING!
    #[connector_test(schema(schema))]
    async fn param_type_changes(runner: Runner) -> TestResult<()> {
        let sql = r#"INSERT INTO "TestModel" (col) VALUES ($1::TEXT::UUID)"#;
        let params_1 = vec![RawParam::Null];
        let params_2 = vec![RawParam::from("71a4d621-8342-4aff-b658-5e65047335b0")];
        run_query!(&runner, fmt_execute_raw(sql, params_1));
        run_query!(&runner, fmt_execute_raw(sql, params_2));

        Ok(())
    }
}
