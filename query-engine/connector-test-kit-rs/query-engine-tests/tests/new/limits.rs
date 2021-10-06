use query_engine_tests::*;

// NOTE: This test depends on the absence of the env variable QUERY_BATCH_SIZE
#[test_suite(schema(schema))]
mod bind_limits {
    use indoc::{formatdoc, indoc};
    use query_engine_tests::assert_query;

    fn schema() -> String {
        let schema = indoc! {
          r#"
          model Test {
            #id(id, Int, @id)
            val1 Int
            val2 Int
            val3 Int
            val4 Int
            val5 Int
            val6 Int
            val7 Int
            val8 Int
            val9 Int
          }"#
        };

        schema.to_owned()
    }

    fn mutation_insert_many(n: u64) -> String {
        // We have 10 fields, fill them with fake data
        let data: Vec<String> = (1..=n)
            .map(|idx| {
                let fields: Vec<String> = (1..=9).map(|i| format!("val{}: {}", i, idx)).collect();
                format!("{{ id: {} {} }}", idx, fields.join(" "))
            })
            .collect();

        let mutation = formatdoc! {r#"
        mutation {{
            createManyTest(data: [
                {data}
            ]) {{ count }}
        }}
        "#, data = data.join(",\n")};

        mutation.to_owned()
    }

    // This is a slow tests, it takes close to 2 minutes to do insert of so many records
    #[connector_test(only(MySQL))]
    async fn mysql(runner: Runner) -> TestResult<()> {
        const LIMIT: u64 = 65535 + 10;

        assert_query!(
            runner,
            mutation_insert_many(LIMIT),
            format!(r#"{{"data":{{"createManyTest":{{"count":{}}}}}}}"#, LIMIT)
        );

        Ok(())
    }

    #[connector_test(only(Postgres))]
    async fn postgres(runner: Runner) -> TestResult<()> {
        const LIMIT: u64 = 32767 + 10;

        assert_query!(
            runner,
            mutation_insert_many(LIMIT),
            format!(r#"{{"data":{{"createManyTest":{{"count":{}}}}}}}"#, LIMIT)
        );

        Ok(())
    }

    #[connector_test(only(SqlServer))]
    async fn mssql(runner: Runner) -> TestResult<()> {
        const LIMIT: u64 = 2099 + 10;

        assert_query!(
            runner,
            mutation_insert_many(LIMIT),
            format!(r#"{{"data":{{"createManyTest":{{"count":{}}}}}}}"#, LIMIT)
        );

        Ok(())
    }

    // NOTE: SQLite does not support createMany, which is weird IMHO
}
