use query_engine_tests::*;

/// Port note: The `findMany` portion of the old `WhereUniqueSpec` was omitted, didn't add any value.
#[test_suite(schema(schemas::user))]
mod where_unique {
    use query_engine_tests::{assert_error, assert_query};

    #[connector_test]
    async fn no_unique_fields(runner: Runner) -> TestResult<()> {
        assert_error!(
            &runner,
            "query { findUniqueUser(where: {}){ id }}",
            2009,
            "Expected a minimum of 1 fields of (id, email, first_name_last_name) to be present, got 0."
        );

        Ok(())
    }

    #[connector_test]
    async fn one_unique_field(runner: Runner) -> TestResult<()> {
        test_users(&runner).await?;
        assert_query!(
            &runner,
            "query { findUniqueUser(where: { id: 1 }){ id }}",
            r#"{"data":{"findUniqueUser":{"id":1}}}"#
        );

        Ok(())
    }

    #[connector_test]
    async fn implicit_unique_and(runner: Runner) -> TestResult<()> {
        test_users(&runner).await?;
        assert_query!(
            &runner,
            "query { findUniqueUser(where: { id: 1 }){ id }}",
            r#"{"data":{"findUniqueUser":{"id":1}}}"#
        );

        Ok(())
    }

    fn schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              unique1 Int @unique
              unique2 Int @unique
              unique3 Int
              unique4 Int

              non_unique Int

              @@unique([unique3, unique4])
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema))]
    async fn where_unique_fails_if_not_unique(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"{ findUniqueTestModel(where: { non_unique: 1 }) { id } }"#,
            2009,
            "Expected a minimum of 1 fields of (id, unique1, unique2, unique3_unique4) to be present, got 0."
        );
        assert_error!(
            runner,
            r#"{ findUniqueTestModel(where: { unique3: 1 }) { id } }"#,
            2009,
            "Expected a minimum of 1 fields of (id, unique1, unique2, unique3_unique4) to be present, got 0."
        );
        assert_error!(
            runner,
            r#"{ findUniqueTestModel(where: { unique4: 1 }) { id } }"#,
            2009,
            "Expected a minimum of 1 fields of (id, unique1, unique2, unique3_unique4) to be present, got 0."
        );
        assert_error!(
            runner,
            r#"{ findUniqueTestModel(where: { AND: [{ id: 1 }] }) { id } }"#,
            2009,
            "Expected a minimum of 1 fields of (id, unique1, unique2, unique3_unique4) to be present, got 0."
        );
        assert_error!(
            runner,
            r#"{ findUniqueTestModel(where: { OR: [{ id: 1 }] }) { id } }"#,
            2009,
            "Expected a minimum of 1 fields of (id, unique1, unique2, unique3_unique4) to be present, got 0."
        );
        assert_error!(
            runner,
            r#"{ findUniqueTestModel(where: { NOT: [{ id: 1 }] }) { id } }"#,
            2009,
            "Expected a minimum of 1 fields of (id, unique1, unique2, unique3_unique4) to be present, got 0."
        );

        Ok(())
    }

    #[connector_test(schema(schema))]
    async fn where_unique_works_if_unique(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation { createOneTestModel(data: { id: 1, unique1: 1, unique2: 1, unique3: 1, unique4: 1, non_unique: 0 }) { id } }"#
        );
        run_query!(
            &runner,
            r#"mutation { createOneTestModel(data: { id: 2, unique1: 1, unique2: 1, unique3: 1, unique4: 1, non_unique: 0 }) { id } }"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findUniqueTestModel(where: { id: 1, non_unique: 0 }) { id } }"#),
          @r###"{"data":{"findUniqueTestModel":{"id":1}}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findUniqueTestModel(where: { unique1: 1, non_unique: 0 }) { id } }"#),
          @r###"{"data":{"findUniqueTestModel":{"id":1}}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findUniqueTestModel(where: { unique2: 1, non_unique: 0 }) { id } }"#),
          @r###"{"data":{"findUniqueTestModel":{"id":1}}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findUniqueTestModel(where: { unique3_unique4: { unique3: 1, unique4: 1 }, non_unique: 0 }) { id } }"#),
          @r###"{"data":{"findUniqueTestModel":{"id":1}}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findUniqueTestModel(where: { id: 1, OR: [{ non_unique: 1 }, { non_unique: 0 }] }) { id } }"#),
          @r###"{"data":{"findUniqueTestModel":{"id":1}}}"###
        );

        Ok(())
    }

    async fn test_users(runner: &Runner) -> TestResult<()> {
        runner
            .query(r#"mutation { createOneUser(data: { id: 1, email: "a@b.com", first_name: "Elongated", last_name: "Muskrat" }) { id } }"#)
            .await?.assert_success();

        runner
            .query(r#"mutation { createOneUser(data: { id: 2, email: "b@a.com", first_name: "John", last_name: "Cena" }) { id } }"#)
            .await?.assert_success();

        Ok(())
    }
}
