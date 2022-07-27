use query_engine_tests::*;

#[test_suite(schema(schemas::user))]
mod find_unique {
    use query_engine_tests::assert_query;

    #[connector_test]
    async fn fetch_unique_by_id(runner: Runner) -> TestResult<()> {
        test_user(&runner).await?;

        assert_query!(
            runner,
            "query { findUniqueUser(where: { id: 1 }) { id } }",
            r#"{"data":{"findUniqueUser":{"id":1}}}"#
        );

        Ok(())
    }

    #[connector_test]
    async fn fetch_unique_by_single_unique(runner: Runner) -> TestResult<()> {
        test_user(&runner).await?;

        assert_query!(
            &runner,
            r#"query { findUniqueUser(where: { email: "a@b.com" }) { id } }"#,
            r#"{"data":{"findUniqueUser":{"id":1}}}"#
        );

        Ok(())
    }

    #[connector_test]
    async fn fetch_unique_by_multi_unique(runner: Runner) -> TestResult<()> {
        test_user(&runner).await?;

        assert_query!(
            &runner,
            r#"query { findUniqueUser(where: { first_name_last_name: { first_name: "Elongated", last_name: "Muskrat" } }) { id } }"#,
            r#"{"data":{"findUniqueUser":{"id":1}}}"#
        );

        Ok(())
    }

    #[connector_test]
    async fn no_result_fetch_unique_by_id(runner: Runner) -> TestResult<()> {
        test_user(&runner).await?;

        assert_query!(
            runner,
            "query { findUniqueUser(where: { id: 2 }) { id } }",
            r#"{"data":{"findUniqueUser":null}}"#
        );

        Ok(())
    }

    #[connector_test]
    async fn no_result_fetch_unique_by_single_unique(runner: Runner) -> TestResult<()> {
        test_user(&runner).await?;

        assert_query!(
            &runner,
            r#"query { findUniqueUser(where: { email: "b@a.com" }) { id } }"#,
            r#"{"data":{"findUniqueUser":null}}"#
        );

        Ok(())
    }

    #[connector_test]
    async fn no_result_fetch_unique_by_multi_unique(runner: Runner) -> TestResult<()> {
        test_user(&runner).await?;

        assert_query!(
            &runner,
            r#"query { findUniqueUser(where: { first_name_last_name: { first_name: "Doesn't", last_name: "Exist" } }) { id } }"#,
            r#"{"data":{"findUniqueUser":null}}"#
        );

        Ok(())
    }

    async fn test_user(runner: &Runner) -> TestResult<()> {
        runner
            .query(r#"mutation { createOneUser(data: { id: 1, email: "a@b.com", first_name: "Elongated", last_name: "Muskrat" }) { id } }"#)
            .await?.assert_success();

        Ok(())
    }

    fn simple_uniq_idx_with_embedded() -> String {
        indoc! {r#"
        type Location {
            address String
        }

        model A {
            #id(id, Int, @id)
            name String
            location Location

            @@unique([location.address])
        }
        "#}
        .to_string()
    }

    #[connector_test(schema(simple_uniq_idx_with_embedded), only(MongoDb))]
    async fn simple_embedded_type(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            indoc! {r#"mutation {
            createManyA(data: [
                {id: 1 name: "foo" location: {set: {address: "a"}}},
                {id: 2 name: "foo" location: {set: {address: "b"}}},
                {id: 3 name: "foo" location: {set: {address: "c"}}},
            ]) { count }
        }"#}
        );

        assert_query!(
            runner,
            r#"query { findUniqueA(where: { 
                location_address: {
                    location: {
                        address: "a"
                    }
                }
            }) { id }}"#,
            r#"{"data":{"findUniqueA":{"id":1}}}"#
        );

        Ok(())
    }

    fn composite_uniq_idx_with_embedded() -> String {
        indoc! {r#"
        type Location {
            address String
        }

        model A {
            #id(id, Int, @id)
            name String
            location Location

            @@unique([name, location.address])
        }
        "#}
        .to_string()
    }

    #[connector_test(schema(composite_uniq_idx_with_embedded), only(MongoDb))]
    async fn composite_embedded_type(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            indoc! {r#"mutation {
            createManyA(data: [
                {id: 1 name: "foo" location: {set: {address: "a"}}},
                {id: 2 name: "foo" location: {set: {address: "b"}}},
                {id: 3 name: "bar" location: {set: {address: "c"}}},
            ]) { count }
        }"#}
        );

        assert_query!(
            runner,
            r#"query { findUniqueA(where: { 
                name_location_address: {
                    name: "foo"
                    location: {
                        address: "a"
                    }
                } 
            }) { id }}"#,
            r#"{"data":{"findUniqueA":{"id":1}}}"#
        );

        Ok(())
    }
}
