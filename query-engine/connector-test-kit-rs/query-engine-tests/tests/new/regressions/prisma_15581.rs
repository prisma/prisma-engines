use query_engine_tests::*;

// issue: https://github.com/prisma/prisma/issues/15581
// notion doc: https://www.notion.so/prismaio/Single-create-with-default-now-or-updatedAt-in-id-fail-d6550337014c4e5ab81d4d362228aa14
// original issue
// https://www.notion.so/prismaio/QE-now-changes-within-the-same-request-280b56d1075f43dea5c5dd82b755541b
// and its issue https://github.com/prisma/prisma/issues/12572
//
// Matching tests for the original issue in prisma_12572.rs
#[test_suite(schema(schema))]
mod prisma_15581 {
    fn schema() -> String {
        r#"
            model test {
                reference Int
                created_at DateTime @default(now())
                other String?

                @@id([reference, created_at])
            }

            model test2 {
                updated_at DateTime @updatedAt
                other String?
                reference Int

                @@id([reference, updated_at])
            }
        "#
        .to_owned()
    }

    #[connector_test(exclude(Mongodb))]
    async fn create_one_model_with_datetime_default_now_in_id(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation { createOnetest(data: { reference: 3 }) { reference created_at other } }"#
        );

        Ok(())
    }

    #[connector_test(exclude(Mongodb))]
    async fn create_one_model_with_updated_at_in_id(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation { createOnetest2(data: { reference: 3 }) { reference updated_at other } }"#
        );

        Ok(())
    }

    fn pg_schema() -> String {
        r#"
            model test {
                reference Int
                created_at DateTime @default(now()) @test.Timestamptz(1)
                other String?

                @@id([reference, created_at])
            }
        "#
        .to_owned()
    }

    #[connector_test(only(Postgres), schema(pg_schema))]
    async fn create_one_model_with_low_precision_datetime_in_id(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation { createOnetest(data: { reference: 3 }) { reference created_at other } }"#
        );

        Ok(())
    }

    fn single_field_id_schema() -> String {
        r#"
            model test {
                #id(created_at, DateTime, @default(now()) @id)
                other String?
            }

            model test2 {
                #id(updated_at, DateTime, @updatedAt @id)
                reference Int
            }
        "#
        .to_owned()
    }

    #[connector_test(schema(single_field_id_schema))]
    async fn single_create_one_model_with_default_now_in_id(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation { createOnetest(data: { other: "meow" }) { created_at other } }"#
        );
        Ok(())
    }

    #[connector_test(schema(single_field_id_schema))]
    async fn single_create_one_model_with_updated_at_in_id(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation { createOnetest2(data: { reference: 2 }) { updated_at reference } }"#
        );
        Ok(())
    }
}
