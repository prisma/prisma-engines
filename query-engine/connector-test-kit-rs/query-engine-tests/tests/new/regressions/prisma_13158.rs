use query_engine_tests::*;

#[test_suite(schema(schema), only(Postgres))] // arrays are postgres-only
mod prisma_1358 {
    fn schema() -> String {
        r#"
            model Test {
                #id(id, String, @id)
                temperatures Float[]
            }

        "#
        .to_owned()
    }

    #[connector_test]
    async fn insert_mixed_int_float_array_in_execute_raw(runner: Runner) -> TestResult<()> {
        let query = fmt_execute_raw(
            r#"INSERT INTO "Test" (id, temperatures) VALUES ($1,$2)"#,
            vec![
                RawParam::from("abc"),
                RawParam::Array(vec![
                    RawParam::from(9.3),
                    RawParam::from(3),
                    RawParam::from(0),
                    RawParam::from(12.99999),
                ]),
            ],
        );
        let result = run_query!(runner, query);
        assert_eq!(result, r#"{"data":{"executeRaw":1}}"#);

        Ok(())
    }
}
