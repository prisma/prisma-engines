use query_engine_tests::*;

#[test_suite(schema(schema), only(MySql("8")))]
mod special_id_values {
    use indoc::indoc;

    fn schema() -> String {
        let prisma = indoc! {r#"
        model BinTest {
            id Bytes @id @default(dbgenerated("(uuid_to_bin(uuid()))")) @test.Binary(16)
            name String
        }
        model StrTest {
            id String @id @default(dbgenerated("(uuid())")) @test.Char(36)
            name String
        }
        model BinStrTest {
            one Bytes @default(dbgenerated("(uuid_to_bin(uuid()))")) @test.Binary(16)
            two String  @default(dbgenerated("(uuid())")) @test.Char(36)
            name String
            @@unique([one, two])
        }
        "#};

        prisma.to_string()
    }

    #[connector_test]
    async fn binary_uuid(runner: Runner) -> TestResult<()> {
        runner
            .query(indoc! {r#"
        mutation {
            createOneBinTest(data: {
                name: "test"
            }) { id }
        }
        "#})
            .await?
            .assert_success();

        Ok(())
    }

    #[connector_test]
    async fn str_uuid(runner: Runner) -> TestResult<()> {
        runner
            .query(indoc! {r#"
        mutation {
            createOneStrTest(data: {
                name: "test"
            }) { id }
        }
        "#})
            .await?
            .assert_success();

        Ok(())
    }

    #[connector_test]
    async fn binary_str_composite(runner: Runner) -> TestResult<()> {
        runner
            .query(indoc! {r#"
        mutation {
            createOneBinStrTest(data: {
                name: "foo"
            }) { one, two }
        }
        "#})
            .await?
            .assert_success();

        Ok(())
    }
}
