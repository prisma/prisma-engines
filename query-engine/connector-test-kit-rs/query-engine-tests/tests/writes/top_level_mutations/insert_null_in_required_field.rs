use query_engine_tests::*;

#[test_suite]
mod insert_null {
    use indoc::indoc;
    use query_engine_tests::{Runner, assert_error, run_query};

    fn schema_1() -> String {
        let schema = indoc! {
            r#"model A {
              #id(id, Int, @id)
              b   String @unique
              key String
            }"#
        };

        schema.to_owned()
    }

    // "Updating a required value to null" should "throw a proper error"
    #[connector_test(schema(schema_1), exclude(MySql(5.6)))]
    async fn update_required_val_to_null(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, b: "abc" key: "abc" }"#).await?;

        if !matches!(
            runner.connector_version(),
            ConnectorVersion::MySql(Some(MySqlVersion::V5_6))
        ) {
            assert_error!(
                runner,
                r#"mutation {
                updateOneA(
                  where: { b: "abc" }
                  data: {
                    key: { set: null }
                  }) {
                  id
                }
              }"#,
                2009,
                "A value is required but not set"
            );
        }

        Ok(())
    }

    fn schema_2() -> String {
        let schema = indoc! {
            r#"model A {
              #id(id, Int, @id)
              b   String  @unique
              key String
            }"#
        };

        schema.to_owned()
    }

    // "Creating a required value as null" should "throw a proper error"
    #[connector_test(schema(schema_2))]
    async fn create_required_value_as_null(runner: Runner) -> TestResult<()> {
        assert_error!(
            &runner,
            r#"mutation {
            createOneA(data: {
              id: 1,
              b: "abc"
              key: null
            }) {
              id
            }
          }"#,
            2009,
            "`data.key`: A value is required but not set"
        );

        Ok(())
    }

    fn schema_3() -> String {
        let schema = indoc! {
            r#"model A {
              #id(id, Int, @id)
              b   String  @unique
              key String? @unique
            }"#
        };

        schema.to_owned()
    }

    // "Updating an optional value to null" should "work"
    #[connector_test(schema(schema_3))]
    async fn update_optional_val_null(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, b: "abc" key: "abc" }"#).await?;

        run_query!(
            &runner,
            r#"mutation {
                updateOneA(
                  where: { b: "abc" }
                  data: {
                    key: { set: null }
                  }) {
                  id
                }
            }"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyA{ b, key }}"#),
          @r###"{"data":{"findManyA":[{"b":"abc","key":null}]}}"###
        );

        Ok(())
    }

    fn schema_4() -> String {
        let schema = indoc! {
            r#"model A {
            #id(id, Int, @id)
            b    String @unique
            key  String?
          }"#
        };

        schema.to_owned()
    }

    // "Creating an optional value as null" should "work"
    #[connector_test(schema(schema_4))]
    async fn create_optional_val_null(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneA(data: {
              id: 1,
              b: "abc"
              key: null
            }) {
              b,
              key
            }
          }"#),
          @r###"{"data":{"createOneA":{"b":"abc","key":null}}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneA(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}
