use query_engine_tests::*;

#[test_suite(schema(schema), only(MySql(8)))]
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
        model SpacesTest {
            id String @id @default(dbgenerated(" ( uuid( ) ) ")) @test.Char(36)
            name String
        }
        model ChoppedTest {
            id String @id @default(dbgenerated("(uuid())"))
            name String
        }
        model BinSwappedTest {
            id Bytes @id @default(dbgenerated("(uuid_to_bin(uuid(), 1))")) @test.Binary(16)
            name String
        }
        model BinNormalTest {
            id Bytes @id @default(dbgenerated("(uuid_to_bin(uuid(), 0))")) @test.Binary(16)
            name String
        }
        "#};

        prisma.to_string()
    }

    #[connector_test]
    async fn binary_uuid(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            indoc! {r#"
        mutation {
            createOneBinTest(data: {
                name: "test"
            }) { id }
        }
        "#}
        );

        Ok(())
    }

    #[connector_test]
    async fn str_uuid(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            indoc! {r#"
        mutation {
            createOneStrTest(data: {
                name: "test"
            }) { id }
        }
        "#}
        );

        Ok(())
    }

    #[connector_test]
    async fn binary_str_composite(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            indoc! {r#"
        mutation {
            createOneBinStrTest(data: {
                name: "foo"
            }) { one, two }
        }
        "#}
        );

        Ok(())
    }

    #[connector_test]
    async fn extra_spaces_are_removed(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            indoc! {r#"
        mutation {
            createOneSpacesTest(data: {
                name: "foo"
            }) { id }
        }
        "#}
        );

        Ok(())
    }

    #[connector_test]
    async fn uuid_without_native_type(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            indoc! {r#"
        mutation {
            createOneChoppedTest(data: {
                name: "foo"
            }) { id }
        }
        "#}
        );

        Ok(())
    }

    #[connector_test]
    async fn uuid_is_provided(runner: Runner) -> TestResult<()> {
        assert_query!(
            runner,
            r#"mutation {
                createOneStrTest(data: {id: "e27861d6-c0cb-4e0b-aac5-158aa6eced65", name: "test"}) {id}
            }"#,
            r#"{"data":{"createOneStrTest":{"id":"e27861d6-c0cb-4e0b-aac5-158aa6eced65"}}}"#
        );

        Ok(())
    }

    #[connector_test]
    async fn uuid_is_swapped(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            indoc! {r#"
        mutation {
            createOneBinSwappedTest(data: {
                name: "foo"
            }) { id }
        }
        "#}
        );

        Ok(())
    }

    #[connector_test]
    async fn uuid_is_normal(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            indoc! {r#"
        mutation {
            createOneBinNormalTest(data: {
                name: "foo"
            }) { id }
        }
        "#}
        );

        Ok(())
    }
}
