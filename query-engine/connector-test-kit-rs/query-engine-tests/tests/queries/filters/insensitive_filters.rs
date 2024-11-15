use query_engine_tests::*;

#[test_suite(schema(schema), capabilities(InsensitiveFilters))]
mod insensitive {
    use indoc::indoc;
    use query_engine_tests::{match_connector_result, run_query};

    fn schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, String, @id, @default(cuid()))
              str String
            }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn string_matchers(runner: Runner) -> TestResult<()> {
        create_row(&runner, "a test").await?;
        create_row(&runner, "A Test").await?;
        create_row(&runner, "b test").await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { str: { startsWith: "a", mode: insensitive } }) { str }}"#),
          @r###"{"data":{"findManyTestModel":[{"str":"a test"},{"str":"A Test"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { str: { endsWith: "Test", mode: insensitive } }) { str }}"#),
          @r###"{"data":{"findManyTestModel":[{"str":"a test"},{"str":"A Test"},{"str":"b test"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { str: { contains: "Te", mode: insensitive } }) { str }}"#),
          @r###"{"data":{"findManyTestModel":[{"str":"a test"},{"str":"A Test"},{"str":"b test"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn neg_string_matchers(runner: Runner) -> TestResult<()> {
        create_row(&runner, "a test").await?;
        create_row(&runner, "A Test").await?;
        create_row(&runner, "b test").await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { str: { not: { startsWith: "a" }, mode: insensitive } }) { str }}"#),
          @r###"{"data":{"findManyTestModel":[{"str":"b test"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { str: { not: { endsWith: "Test" }, mode: insensitive } }) { str }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { str: { not: { contains: "Te" }, mode: insensitive } }) { str }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn numeric_matchers(runner: Runner) -> TestResult<()> {
        create_row(&runner, "a").await?;
        create_row(&runner, "A").await?;
        create_row(&runner, "b").await?;

        // gt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { str: { gt: "a", mode: insensitive } }) { str }}"#),
          @r###"{"data":{"findManyTestModel":[{"str":"b"}]}}"###
        );

        // not gt => lte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { str: { not: { gt: "a" }, mode: insensitive  } }) { str }}"#),
          @r###"{"data":{"findManyTestModel":[{"str":"a"},{"str":"A"}]}}"###
        );

        // gte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { str: { gte: "a", mode: insensitive } }) { str }}"#),
          @r###"{"data":{"findManyTestModel":[{"str":"a"},{"str":"A"},{"str":"b"}]}}"###
        );

        // not gte => lt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { str: { not: { gte: "a" }, mode: insensitive  } }) { str }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        // lt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { str: { lt: "a", mode: insensitive } }) { str }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        // not lt => gte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { str: { not: { lt: "a" }, mode: insensitive  } }) { str }}"#),
          @r###"{"data":{"findManyTestModel":[{"str":"a"},{"str":"A"},{"str":"b"}]}}"###
        );

        // lte
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { str: { lte: "a", mode: insensitive } }) { str }}"#),
          @r###"{"data":{"findManyTestModel":[{"str":"a"},{"str":"A"}]}}"###
        );

        // not lte => gt
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { str: { not: { lte: "a" }, mode: insensitive  } }) { str }}"#),
          @r###"{"data":{"findManyTestModel":[{"str":"b"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn comparator_ops(runner: Runner) -> TestResult<()> {
        // Note: Postgres collations order characters differently than, say, using .sort in most programming languages,
        // which is why the results of <, >, etc. are non-obvious at a glance.

        create_row(&runner, "A").await?;
        create_row(&runner, "æ").await?;
        create_row(&runner, "Æ").await?;
        create_row(&runner, "bar").await?;
        create_row(&runner, "aÆB").await?;
        create_row(&runner, "AÆB").await?;
        create_row(&runner, "aæB").await?;
        create_row(&runner, "aB").await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { str: { equals: "æ", mode: insensitive } }) { str }}"#),
          @r###"{"data":{"findManyTestModel":[{"str":"æ"},{"str":"Æ"}]}}"###
        );

        match_connector_result!(
          &runner,
          r#"query { findManyTestModel(where: { str: { gte: "aÆB", mode: insensitive } }) { str }}"#,
          MongoDb(_) => vec![r#"{"data":{"findManyTestModel":[{"str":"æ"},{"str":"Æ"},{"str":"bar"},{"str":"aÆB"},{"str":"AÆB"},{"str":"aæB"}]}}"#],
          // Cockroach, https://github.com/cockroachdb/cockroach/issues/71313
          CockroachDb(_) => vec![r#"{"data":{"findManyTestModel":[{"str":"æ"},{"str":"Æ"},{"str":"bar"},{"str":"aÆB"},{"str":"AÆB"},{"str":"aæB"}]}}"#],
          _ => vec![r#"{"data":{"findManyTestModel":[{"str":"æ"},{"str":"Æ"},{"str":"bar"},{"str":"aÆB"},{"str":"AÆB"},{"str":"aæB"},{"str":"aB"}]}}"#]
        );

        match_connector_result!(
          &runner,
          r#"query { findManyTestModel(where: { str: { lt: "aÆB", mode: insensitive } }) { str }}"#,
          MongoDb(_) => vec![r#"{"data":{"findManyTestModel":[{"str":"A"},{"str":"aB"}]}}"#],
          // https://github.com/cockroachdb/cockroach/issues/71313
          CockroachDb(_) => vec![r#"{"data":{"findManyTestModel":[{"str":"A"},{"str":"aB"}]}}"#],
          _ =>  vec![r#"{"data":{"findManyTestModel":[{"str":"A"}]}}"#]
        );

        Ok(())
    }

    #[connector_test]
    async fn list_containment_ops(runner: Runner) -> TestResult<()> {
        create_row(&runner, "A").await?;
        create_row(&runner, "æ").await?;
        create_row(&runner, "Æ").await?;
        create_row(&runner, "b").await?;
        create_row(&runner, "B").await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { str: { in: ["æ", "b"], mode: insensitive } }) { str }}"#),
          @r###"{"data":{"findManyTestModel":[{"str":"æ"},{"str":"Æ"},{"str":"b"},{"str":"B"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { str: { not: { in: ["æ", "b"] }, mode: insensitive } }) { str }}"#),
          @r###"{"data":{"findManyTestModel":[{"str":"A"}]}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, s: &str) -> TestResult<()> {
        runner
            .query(format!(
                r#"mutation {{ createOneTestModel(data: {{ str: "{s}" }}) {{ id }} }}"#
            ))
            .await?
            .assert_success();

        Ok(())
    }
}
