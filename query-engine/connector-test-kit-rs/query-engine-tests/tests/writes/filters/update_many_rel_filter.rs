use query_engine_tests::*;

#[test_suite(schema(schema), exclude(SqlServer))]
mod update_many_rel_filter {
    use indoc::indoc;
    use query_engine_tests::run_query;

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

    // "The updateMany Mutation" should "update the items matching the where relation filter"
    // TODO(dom): Not working on Mongo (nothing seems to be updated)
    #[connector_test(exclude(SqlServer, MongoDb))]
    async fn update_items_matching_where_rel_filter(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, top: "top1"}"#).await?;
        create_row(runner, r#"{ id: 2, top: "top2"}"#).await?;
        create_row(
            runner,
            r#"{
                  id: 3,
                  top: "top3"
                  bottom: {
                    create: {id: 1, bottom: "bottom1"}
                  }
              }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTop(where: { bottom: { is: null } }) { id } }"#),
          @r###"{"data":{"findManyTop":[{"id":1},{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateManyTop(
              where: { bottom: { is: null } }
              data: { top: { set: "updated" } }
            ) { count } }
          "#),
          @r###"{"data":{"updateManyTop":{"count":2}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTop(where: { bottom: { is: null } }) { id top } }"#),
          @r###"{"data":{"findManyTop":[{"id":1,"top":"updated"},{"id":2,"top":"updated"}]}}"###
        );

        Ok(())
    }

    //"The updateMany Mutation" should "update all items if the filter is empty"
    #[connector_test]
    async fn update_all_items_if_filter_empty(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, top: "top1"}"#).await?;
        create_row(runner, r#"{ id: 2, top: "top2"}"#).await?;
        create_row(
            runner,
            r#"{
                id: 3,
                top: "top3"
                bottom: {
                  create: {id: 1, bottom: "bottom1"}
                }
            }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {updateManyTop(data: { top: { set: "updated" }}){count}}"#),
          @r###"{"data":{"updateManyTop":{"count":3}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTop(where: { top: { equals: "updated" }}) { id top } }"#),
          @r###"{"data":{"findManyTop":[{"id":1,"top":"updated"},{"id":2,"top":"updated"},{"id":3,"top":"updated"}]}}"###
        );

        Ok(())
    }

    // "The updateMany Mutation" should "work for deeply nested filters"
    // TODO(dom): Not working on mongo
    #[connector_test(exclude(SqlServer, MongoDb))]
    async fn works_with_deeply_nested_filters(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, top: "top1"}"#).await?;
        create_row(runner, r#"{ id: 2, top: "top2"}"#).await?;
        create_row(
            runner,
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

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTop(where: { bottom: { is: { veryBottom: { is: { veryBottom: { equals: "veryBottom" }}}}}}) { id } }"#),
          @r###"{"data":{"findManyTop":[{"id":3}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateManyTop(
              where: { bottom: { is: { veryBottom: { is: { veryBottom: { equals: "veryBottom" }}}}}}
              data: { top: { set: "updated" } }
            ) { count } }
          "#),
          @r###"{"data":{"updateManyTop":{"count":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTop(where: { top: { equals: "updated" }}) { id top } }"#),
          @r###"{"data":{"findManyTop":[{"id":3,"top":"updated"}]}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTop(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
