use query_engine_tests::*;

#[test_suite(schema(schema))]
mod order_by_dependent {
    use indoc::indoc;
    use query_engine_tests::{assert_query_many, run_query};

    fn schema() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, Int, @id)
              b_id Int?
              b    ModelB? @relation(fields: [b_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
              c    ModelC?
            }

            model ModelB {
              #id(id, Int, @id)
              a  ModelA?

              c_id Int?
              c    ModelC? @relation(fields: [c_id], references: [id])
            }

            model ModelC {
              #id(id, Int, @id)
              b    ModelB?
              a_id Int?
              a    ModelA? @relation(fields: [a_id], references: [id])
            }"#
        };

        schema.to_owned()
    }

    // "[Hops: 1] Ordering by related record field ascending" should "work"
    #[connector_test(exclude(SqlServer))]
    async fn hop_1_related_record_asc(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, Some(2), Some(3), None).await?;
        create_row(&runner, 4, Some(5), Some(6), None).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyModelA(orderBy: { b: { id: asc }}) {
              id
              b {
                id
              }
            }
          }"#),
          @r###"{"data":{"findManyModelA":[{"id":1,"b":{"id":2}},{"id":4,"b":{"id":5}}]}}"###
        );

        Ok(())
    }

    // "[Hops: 1] Ordering by related record field descending" should "work"
    #[connector_test(exclude(SqlServer))]
    async fn hop_1_related_record_desc(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, Some(2), Some(3), None).await?;
        create_row(&runner, 4, Some(5), Some(6), None).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyModelA(orderBy: { b: { id: desc }}) {
              id
              b {
                id
              }
            }
          }"#),
          @r###"{"data":{"findManyModelA":[{"id":4,"b":{"id":5}},{"id":1,"b":{"id":2}}]}}"###
        );

        Ok(())
    }

    // "[Hops: 1] Ordering by related record field ascending with nulls" should "work"
    #[connector_test]
    async fn hop_1_related_record_asc_nulls(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, Some(1), Some(1), None).await?;
        create_row(&runner, 2, Some(2), None, None).await?;
        create_row(&runner, 3, None, None, None).await?;

        assert_query_many!(
            &runner,
            r#"{
              findManyModelA(orderBy: { b: { id: asc }}) {
                id
                b {
                  id
                }
              }
            }"#,
            vec![
                r#"{"data":{"findManyModelA":[{"id":3,"b":null},{"id":1,"b":{"id":1}},{"id":2,"b":{"id":2}}]}}"#,
                r#"{"data":{"findManyModelA":[{"id":1,"b":{"id":1}},{"id":2,"b":{"id":2}},{"id":3,"b":null}]}}"#
            ]
        );

        Ok(())
    }

    // "[Hops: 2] Ordering by related record field ascending" should "work"
    #[connector_test(exclude(SqlServer))]
    async fn hop_2_related_record_asc(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, Some(2), Some(3), None).await?;
        create_row(&runner, 4, Some(5), Some(6), None).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
                findManyModelA(orderBy: { b: { c: { id: asc }}}) {
                  id
                  b { c { id }}
                }
              }"#),
          @r###"{"data":{"findManyModelA":[{"id":1,"b":{"c":{"id":3}}},{"id":4,"b":{"c":{"id":6}}}]}}"###
        );

        Ok(())
    }

    // "[Hops: 2] Ordering by related record field descending" should "work"
    #[connector_test(exclude(SqlServer))]
    async fn hop_2_related_record_desc(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, Some(2), Some(3), None).await?;
        create_row(&runner, 4, Some(5), Some(6), None).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyModelA(orderBy: { b: { c: { id: desc }}}) {
              id
              b { c { id }}
            }
          }"#),
          @r###"{"data":{"findManyModelA":[{"id":4,"b":{"c":{"id":6}}},{"id":1,"b":{"c":{"id":3}}}]}}"###
        );

        Ok(())
    }

    // "[Hops: 2] Ordering by related record field ascending with nulls" should "work"
    #[connector_test]
    async fn hop_2_related_record_asc_null(runner: Runner) -> TestResult<()> {
        // 1 record has the "full chain", one half, one none
        create_row(&runner, 1, Some(1), Some(1), None).await?;
        create_row(&runner, 2, Some(2), None, None).await?;
        create_row(&runner, 3, None, None, None).await?;

        assert_query_many!(
            &runner,
            r#"{
              findManyModelA(orderBy: { b: { c: { id: asc }}}) {
                id
                b {
                  c {
                    id
                  }
                }
              }
            }"#,
            // Depends on how null values are handled.
            vec![
                r#"{"data":{"findManyModelA":[{"id":2,"b":{"c":null}},{"id":3,"b":null},{"id":1,"b":{"c":{"id":1}}}]}}"#,
                r#"{"data":{"findManyModelA":[{"id":3,"b":null},{"id":2,"b":{"c":null}},{"id":1,"b":{"c":{"id":1}}}]}}"#,
                r#"{"data":{"findManyModelA":[{"id":1,"b":{"c":{"id":1}}},{"id":2,"b":{"c":null}},{"id":3,"b":null}]}}"#,
                // CockroachDB can order ModelA.id in any order if ModelC.b_id is NULL.
                r#"{"data":{"findManyModelA":[{"id":1,"b":{"c":{"id":1}}},{"id":3,"b":null},{"id":2,"b":{"c":null}}]}}"#,
            ]
        );

        Ok(())
    }

    // "[Circular] Ordering by related record field ascending" should "work"
    #[connector_test]
    async fn circular_related_record_asc(runner: Runner) -> TestResult<()> {
        // Records form circles with their relations
        create_row(&runner, 1, Some(1), Some(1), Some(1)).await?;
        create_row(&runner, 2, Some(2), Some(2), Some(2)).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyModelA(orderBy: { b: { c: { a: { id: asc }}}}) {
              id
              b {
                c {
                  a {
                    id
                  }
                }
              }
            }
          }"#),
          @r###"{"data":{"findManyModelA":[{"id":1,"b":{"c":{"a":{"id":1}}}},{"id":2,"b":{"c":{"a":{"id":2}}}}]}}"###
        );

        Ok(())
    }

    // "[Circular] Ordering by related record field descending" should "work"
    #[connector_test]
    async fn circular_related_record_desc(runner: Runner) -> TestResult<()> {
        // Records form circles with their relations
        create_row(&runner, 1, Some(1), Some(1), Some(1)).await?;
        create_row(&runner, 2, Some(2), Some(2), Some(2)).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyModelA(orderBy: { b: { c: { a: { id: desc }}}}) {
              id
              b {
                c {
                  a {
                    id
                  }
                }
              }
            }
          }"#),
          @r###"{"data":{"findManyModelA":[{"id":2,"b":{"c":{"a":{"id":2}}}},{"id":1,"b":{"c":{"a":{"id":1}}}}]}}"###
        );

        Ok(())
    }

    // "[Circular with differing records] Ordering by related record field ascending" should "work"
    #[connector_test(exclude(SqlServer, MySql))]
    async fn circular_diff_related_record_asc(runner: Runner) -> TestResult<()> {
        // Records form circles with their relations
        create_row(&runner, 1, Some(1), Some(1), Some(3)).await?;
        create_row(&runner, 2, Some(2), Some(2), Some(4)).await?;

        assert_query_many!(
            &runner,
            r#"{
              findManyModelA(orderBy: { b: { c: { a: { id: asc }}}}) {
                id
                b {
                  c {
                    a {
                      id
                    }
                  }
                }
              }
            }"#,
            // Depends on how null values are handled.
            vec![
                r#"{"data":{"findManyModelA":[{"id":3,"b":null},{"id":4,"b":null},{"id":1,"b":{"c":{"a":{"id":3}}}},{"id":2,"b":{"c":{"a":{"id":4}}}}]}}"#,
                r#"{"data":{"findManyModelA":[{"id":1,"b":{"c":{"a":{"id":3}}}},{"id":2,"b":{"c":{"a":{"id":4}}}},{"id":3,"b":null},{"id":4,"b":null}]}}"#,
            ]
        );

        Ok(())
    }

    // "[Circular with differing records] Ordering by related record field descending" should "work"
    #[connector_test(exclude(SqlServer, MySql))]
    async fn circular_diff_related_record_desc(runner: Runner) -> TestResult<()> {
        // Records form circles with their relations
        create_row(&runner, 1, Some(1), Some(1), Some(3)).await?;
        create_row(&runner, 2, Some(2), Some(2), Some(4)).await?;

        assert_query_many!(
            &runner,
            r#"{
              findManyModelA(orderBy: { b: { c: { a: { id: desc }}}}) {
                id
                b {
                  c {
                    a {
                      id
                    }
                  }
                }
              }
            }"#,
            // Depends on how null values are handled.
            vec![
                r#"{"data":{"findManyModelA":[{"id":2,"b":{"c":{"a":{"id":4}}}},{"id":1,"b":{"c":{"a":{"id":3}}}},{"id":3,"b":null},{"id":4,"b":null}]}}"#,
                r#"{"data":{"findManyModelA":[{"id":3,"b":null},{"id":4,"b":null},{"id":2,"b":{"c":{"a":{"id":4}}}},{"id":1,"b":{"c":{"a":{"id":3}}}}]}}"#,
            ]
        );

        Ok(())
    }

    fn multiple_rel_same_model() -> String {
        let schema = indoc! {
          r#"model ModelA {
          #id(id, Int, @id)

          b1_id Int?
          b1    ModelB? @relation(fields: [b1_id], references: [id], name: "1", onDelete: NoAction, onUpdate: NoAction)

          b2_id Int?
          b2    ModelB? @relation(fields: [b2_id], references: [id], name: "2")
        }

        model ModelB {
          #id(id, Int, @id)

          a1 ModelA[] @relation("1")
          a2 ModelA[] @relation("2")
        }"#
        };

        schema.to_string()
    }

    #[connector_test(schema(multiple_rel_same_model))]
    async fn multiple_rel_same_model_order_by(runner: Runner) -> TestResult<()> {
        // test data
        run_query!(
            &runner,
            r#"mutation { createOneModelA(data: { id: 1, b1: { create: { id: 1 } }, b2: { create: { id: 10 } } }) { id }}"#
        );
        run_query!(
            &runner,
            r#"mutation { createOneModelA(data: { id: 2, b1: { connect: { id: 1 } }, b2: { create: { id: 5 } } }) { id }}"#
        );
        run_query!(
            &runner,
            r#"mutation { createOneModelA(data: { id: 3, b1: { create: { id: 2 } }, b2: { create: { id: 7 } } }) { id }}"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyModelA(orderBy: [{ b1: { id: asc } }, { b2: { id: desc } }]) {
              id
              b1 { id }
              b2 { id }
            }
          }"#),
          @r###"{"data":{"findManyModelA":[{"id":1,"b1":{"id":1},"b2":{"id":10}},{"id":2,"b1":{"id":1},"b2":{"id":5}},{"id":3,"b1":{"id":2},"b2":{"id":7}}]}}"###
        );

        Ok(())
    }

    fn no_double_nulls() -> String {
        let schema = indoc! {
          r#"
        model ModelA {
          #id(id, Int, @id)
          b_id Int?
          b    ModelB? @relation(fields: [b_id], references: [id])
        }

        model ModelB {
          #id(id, Int, @id)
          a  ModelA?

          c_id Int?
          c    ModelC? @relation(fields: [c_id], references: [id])
        }

        model ModelC {
          #id(id, Int, @id)
          b    ModelB?
        }
        "#
        };

        schema.to_string()
    }

    // Minimal tests specifically for covering the basics in SQL server (no double nulls allowed).
    #[connector_test(schema(no_double_nulls))]
    async fn simple_order_by_rel(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, Some(2), Some(3), None).await?;
        create_row(&runner, 4, Some(5), Some(7), None).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyModelA(orderBy: { b: { id: asc }}) {
              id
              b {
                id
              }
            }
          }"#),
          @r###"{"data":{"findManyModelA":[{"id":1,"b":{"id":2}},{"id":4,"b":{"id":5}}]}}"###
        );

        Ok(())
    }

    // Minimal tests specifically for covering the basics in SQL server (no double nulls allowed).
    #[connector_test(schema(no_double_nulls))]
    async fn hop_2_simple_order_by_rel(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, Some(2), Some(3), None).await?;
        create_row(&runner, 4, Some(5), Some(6), None).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyModelA(orderBy: { b: { c: { id: asc }}}) {
              id
              b { c { id }}
            }
          }"#),
          @r###"{"data":{"findManyModelA":[{"id":1,"b":{"c":{"id":3}}},{"id":4,"b":{"c":{"id":6}}}]}}"###
        );

        Ok(())
    }

    async fn create_row(
        runner: &Runner,
        a_id: u32,
        b_id: Option<u32>,
        c_id: Option<u32>,
        c_to_a: Option<u32>,
    ) -> TestResult<()> {
        let (follow_up, inline) = match c_to_a {
            Some(id) if id != a_id => (None, Some(format!("a: {{ create: {{ id: {} }} }}", id))),
            Some(id) => (
                Some(format!(
                    "mutation {{ updateOneModelC(where: {{ id: {} }}, data: {{ a_id: {} }}) {{ id }} }}",
                    c_id.unwrap(),
                    id
                )),
                None,
            ),
            None => (None, None),
        };

        let model_c = match c_id {
            Some(id) => format!(
                "c: {{ create: {{ id: {} \n {} }} }}",
                id,
                inline.unwrap_or_else(|| "".to_string())
            ),
            None => "".to_string(),
        };

        let model_b = match b_id {
            Some(id) => format!("b: {{ create: {{ id: {}\n {} }} }}", id, model_c),
            None => "".to_string(),
        };

        let model_a = format!("{{ id: {} \n {} }}", a_id, model_b);

        runner
            .query(format!("mutation {{ createOneModelA(data: {}) {{ id }} }}", model_a))
            .await?
            .assert_success();

        if let Some(query) = follow_up {
            runner.query(query).await?.assert_success();
        };

        Ok(())
    }
}
