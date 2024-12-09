use query_engine_tests::*;

#[test_suite(schema(schema))]
mod logs {
    use indoc::indoc;
    use query_core::executor::TraceParent;

    fn schema() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, Int, @id)
              bs ModelB[]
            }

            model ModelB {
              #id(id, Int, @id)
              str1 String
              str2 String?
              str3 String? @default("SOME_DEFAULT")
              a_id Int?
              a    ModelA? @relation(fields: [a_id], references: [id])
            }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn nested_read_logs_all_have_traceparent(mut runner: Runner) -> TestResult<()> {
        let traceparent = TraceParent::new_random();

        runner
            .query_with_traceparent(
                traceparent,
                r#"{
                    findManyModelA {
                      id
                      bs { id }
                    }
                }"#,
            )
            .await?
            .assert_success();

        assert_all_logs_contain_tracespans(&mut runner, traceparent).await
    }

    #[connector_test]
    async fn nested_create_logs_all_have_traceparent(mut runner: Runner) -> TestResult<()> {
        let traceparent = TraceParent::new_random();
        runner
            .query_with_traceparent(
                traceparent,
                r#"mutation {
                  createOneModelA(data: {
                    id: 1,
                    bs: {
                      createMany: {
                        data: [
                          { id: 1, str1: "1", str2: "1", str3: "1"},
                          { id: 2, str1: "2",            str3: null},
                          { id: 3, str1: "1"},
                        ]
                      }
                    }
                  }) {
                    bs { id }
                  }
                }"#,
            )
            .await?
            .assert_success();

        assert_all_logs_contain_tracespans(&mut runner, traceparent).await
    }

    #[connector_test]
    async fn nested_update_logs_all_have_traceparent(mut runner: Runner) -> TestResult<()> {
        let traceparent = TraceParent::new_random();
        runner
            .query_with_traceparent(
                traceparent,
                r#"mutation {
                  createOneModelA(data: {
                    id: 1,
                    bs: {
                      create: { id: 1, str1: "1", str2: "1", str3: "1" }
                    }
                  }) { id }
                }"#,
            )
            .await?
            .assert_success();

        runner
            .query_with_traceparent(
                traceparent,
                r#"mutation {
                  updateOneModelA(
                    where: {
                        id: 1
                    }
                    data: {
                      bs: {
                        updateMany: {
                          where: { id: 1 }
                          data: { str1: { set: "updated" } }
                        }
                      }
                    }
                  ) {
                    bs { id }
                  }
                }"#,
            )
            .await?
            .assert_success();

        assert_all_logs_contain_tracespans(&mut runner, traceparent).await
    }

    #[connector_test]
    async fn nested_delete_in_update_logs_all_have_traceparent(mut runner: Runner) -> TestResult<()> {
        let traceparent = TraceParent::new_random();
        runner
            .query_with_traceparent(
                traceparent,
                r#"mutation {
                  createOneModelA(data: {
                    id: 1,
                    bs: {
                      create: { id: 1, str1: "1", str2: "1", str3: "1" }
                    }
                  }) { id }
                }"#,
            )
            .await?
            .assert_success();

        runner
            .query_with_traceparent(
                traceparent,
                r#"mutation {
                  updateOneModelA(
                    where: {
                        id: 1
                    }
                    data: {
                      bs: {
                        delete: { id: 1 }
                      }
                    }
                  ) {
                    bs { id }
                  }
                }"#,
            )
            .await?
            .assert_success();

        assert_all_logs_contain_tracespans(&mut runner, traceparent).await
    }

    async fn assert_all_logs_contain_tracespans(runner: &mut Runner, traceparent: TraceParent) -> TestResult<()> {
        let logs = runner.get_logs().await;

        let query_logs = logs
            .iter()
            .filter(|log| {
                log.split_once("db.statement=")
                    .is_some_and(|(_, q)| !q.starts_with("BEGIN") && !q.starts_with("COMMIT"))
            })
            .collect::<Vec<_>>();
        assert!(query_logs.len() > 0, "expected db.statement logs in {logs:?}");

        let expected_traceparent = format!("/* traceparent='{}' */", traceparent);
        let mismatching = query_logs
            .iter()
            .filter(|log| !log.contains(&expected_traceparent))
            .collect::<Vec<_>>();
        assert!(
            mismatching.is_empty(),
            "{expected_traceparent} not found in {mismatching:?}",
        );

        Ok(())
    }
}
