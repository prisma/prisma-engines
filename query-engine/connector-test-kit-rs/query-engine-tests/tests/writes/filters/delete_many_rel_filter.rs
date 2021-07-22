use query_engine_tests::*;

#[test_suite(schema(schema), exclude(SqlServer))]
mod delete_many_rel_filter {
    use indoc::indoc;
    use query_engine_tests::{run_query, run_query_json};

    fn schema() -> String {
        let schema = indoc! {
            r#"model Top{
              #id(id, Int, @id)
              top      String
              bottomId Int?

              bottom Bottom? @relation(fields: [bottomId], references: [id])
           }

           model Bottom{
              #id(id, Int, @id)
              bottom       String
              veryBottomId Int?

              top        Top?
              veryBottom VeryBottom? @relation(fields: [veryBottomId], references: [id])
           }

           model VeryBottom{
              #id(id, Int, @id)
              veryBottom String
              bottom     Bottom?
           }"#
        };

        schema.to_owned()
    }

    // "The delete many Mutation" should "delete the items matching the where relation filter"
    #[connector_test(exclude(SqlServer))]
    async fn delete_items_matching_where_rel_filter(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, top: "top1"}"#).await?;
        create_row(&runner, r#"{ id: 2, top: "top2"}"#).await?;
        create_row(
            &runner,
            r#"{
                  id: 3,
                  top: "top3"
                  bottom: {
                    create: {id: 1, bottom: "bottom1"}
                  }
              }"#,
        )
        .await?;

        assert_eq!(top_count(&runner).await?, 3);

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTop(where: { bottom: { is: null } }) { id } }"#),
          @r###"{"data":{"findManyTop":[{"id":1},{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { deleteManyTop(where: { bottom: { is: null } }) { count } }"#),
          @r###"{"data":{"deleteManyTop":{"count":2}}}"###
        );

        assert_eq!(top_count(&runner).await?, 1);

        Ok(())
    }

    #[connector_test]
    async fn delete_all_items_if_filter_empty(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, top: "top1"}"#).await?;
        create_row(&runner, r#"{ id: 2, top: "top2"}"#).await?;
        create_row(
            &runner,
            r#"{
                id: 3,
                top: "top3"
                bottom: {
                  create: {id: 1, bottom: "bottom1"}
                }
            }"#,
        )
        .await?;

        assert_eq!(top_count(&runner).await?, 3);

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {deleteManyTop{count}}"#),
          @r###"{"data":{"deleteManyTop":{"count":3}}}"###
        );

        assert_eq!(top_count(&runner).await?, 0);

        Ok(())
    }

    // "The delete many Mutation" should "work for deeply nested filters"
    #[connector_test(exclude(SqlServer))]
    async fn works_with_deeply_nested_filters(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, top: "top1"}"#).await?;
        create_row(&runner, r#"{ id: 2, top: "top2"}"#).await?;
        create_row(
            &runner,
            r#"{
                id: 3,
                top: "top3"
                bottom: {
                  create: {
                    id: 1,
                    bottom: "bottom1",
                    veryBottom: {create: {id: 1, veryBottom: "veryBottom"}}
                  }
                }
            }"#,
        )
        .await?;

        assert_eq!(top_count(&runner).await?, 3);

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTop(where: { bottom: { is: { veryBottom: { is: { veryBottom: { equals: "veryBottom" }}}}}}) { id } }"#),
          @r###"{"data":{"findManyTop":[{"id":3}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { deleteManyTop(where: { bottom: { is: { veryBottom: { is: { veryBottom: { equals: "veryBottom" }}}}}}) { count } }"#),
          @r###"{"data":{"deleteManyTop":{"count":1}}}"###
        );

        assert_eq!(top_count(&runner).await?, 2);

        Ok(())
    }

    async fn top_count(runner: &Runner) -> TestResult<usize> {
        let res = run_query_json!(runner, r#"{ findManyTop { id } }"#);
        let tops = &res["data"]["findManyTop"];
        match tops {
            serde_json::Value::Array(arr) => Ok(arr.len()),
            _ => unreachable!(),
        }
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTop(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
