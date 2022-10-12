use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema))]
mod where_unique {
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

    #[connector_test]
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

    #[connector_test]
    async fn where_unique_works_if_unique(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation { createOneTestModel(data: { id: 1, unique1: 1, unique2: 1, unique3: 1, unique4: 1, non_unique: 0 }) { id } }"#
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
}
