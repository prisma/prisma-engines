use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema), only(Postgres))]
mod raw_errors {
    fn schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              point Unsupported("point")
            }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn unsupported_columns(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            fmt_execute_raw(
                r#"INSERT INTO "TestModel" ("id", "point") VALUES (1, Point(1,2));"#,
                vec![],
            )
        );

        assert_error!(
            runner,
            fmt_query_raw(r#"SELECT * FROM "TestModel";"#, vec![]),
            2010,
            "Failed to deserialize column of type 'point'. If you're using $queryRaw and this column is explicitly marked as `Unsupported` in your Prisma schema, try casting this column to any supported Prisma type such as `String`."
        );

        Ok(())
    }

    #[connector_test(schema(common_nullable_types), only(Postgres), exclude(JS))]
    async fn list_param_for_scalar_column_should_not_panic_quaint(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            fmt_execute_raw(
                r#"INSERT INTO "TestModel" ("id") VALUES ($1);"#,
                vec![RawParam::array(vec![1])],
            ),
            2010,
            r#"column "id" is of type integer but expression is of type bigint[]"#
        );

        Ok(())
    }

    #[connector_test(schema(common_nullable_types), only(JS, Postgres))]
    async fn list_param_for_scalar_column_should_not_panic_pg_js(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            fmt_execute_raw(
                r#"INSERT INTO "TestModel" ("id") VALUES ($1);"#,
                vec![RawParam::array(vec![1])],
            ),
            2010,
            r#"invalid input syntax for type integer"#
        );

        Ok(())
    }
}
