use query_engine_tests::*;

async fn on_unknown_field(runner: Runner) -> TestResult<()> {
    create_test_data(&runner).await?;

    assert_error!(
        &runner,
        r#"{ findManyTestModel(orderBy: { _relevance: { fields: unknown, search: "developer", sort: desc } }) { id } }"#,
        2009,
        "Unable to match input value to any allowed input type for the field"
    );

    Ok(())
}

async fn on_single_field(runner: Runner) -> TestResult<()> {
    create_test_data(&runner).await?;

    match_connector_result!(
      &runner,
      r#"{ findManyTestModel(orderBy: { _relevance: { fields: fieldA, search: "developer", sort: desc } }) { id } }"#,
      // For MySql id 3 and id 1 row have the same ranking score so they are switched between position 2 and 3
      MySql(_) => vec![r#"{"data":{"findManyTestModel":[{"id":2},{"id":3},{"id":1}]}}"#, r#"{"data":{"findManyTestModel":[{"id":2},{"id":1},{"id":3}]}}"#],
      _ => vec![r#"{"data":{"findManyTestModel":[{"id":2},{"id":1},{"id":3}]}}"#]
    );

    match_connector_result!(
      &runner,
      r#"{ findManyTestModel(orderBy: { _relevance: { fields: fieldA, search: "developer", sort: asc } }) { id } }"#,
      MySql(_) => vec![r#"{"data":{"findManyTestModel":[{"id":1},{"id":3},{"id":2}]}}"#, r#"{"data":{"findManyTestModel":[{"id":3},{"id":1},{"id":2}]}}"#],
      _ => vec![r#"{"data":{"findManyTestModel":[{"id":1},{"id":3},{"id":2}]}}"#]
    );

    Ok(())
}

async fn on_single_nullable_field(runner: Runner) -> TestResult<()> {
    create_test_data(&runner).await?;

    match_connector_result!(
      &runner,
      r#"{ findManyTestModel(orderBy: { _relevance: { fields: fieldC, search: "developer", sort: desc } }) { id } }"#,
      MySql(_) => vec![r#"{"data":{"findManyTestModel":[{"id":3},{"id":1},{"id":2}]}}"#, r#"{"data":{"findManyTestModel":[{"id":3},{"id":2},{"id":1}]}}"#],
      _ => vec![r#"{"data":{"findManyTestModel":[{"id":3},{"id":1},{"id":2}]}}"#]
    );

    match_connector_result!(
      &runner,
      r#"{ findManyTestModel(orderBy: { _relevance: { fields: fieldC, search: "developer", sort: asc } }) { id } }"#,
      MySql(_) => vec![r#"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3}]}}"#, r#"{"data":{"findManyTestModel":[{"id":2},{"id":1},{"id":3}]}}"#],
      _ => vec![r#"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3}]}}"#]
    );

    Ok(())
}

async fn on_many_fields(runner: Runner) -> TestResult<()> {
    create_test_data(&runner).await?;

    insta::assert_snapshot!(
      run_query!(&runner, r#"{ findManyTestModel(orderBy: { _relevance: { fields: [fieldA, fieldB], search: "developer", sort: desc } }) { id } }"#),
      @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3}]}}"###
    );

    insta::assert_snapshot!(
      run_query!(&runner, r#"{ findManyTestModel(orderBy: { _relevance: { fields: [fieldA, fieldB], search: "developer", sort: asc } }) { id } }"#),
      @r###"{"data":{"findManyTestModel":[{"id":3},{"id":2},{"id":1}]}}"###
    );

    Ok(())
}

async fn on_many_fields_some_nullable(runner: Runner) -> TestResult<()> {
    create_test_data(&runner).await?;

    insta::assert_snapshot!(
      run_query!(&runner, r#"{ findManyTestModel(orderBy: { _relevance: { fields: [fieldB, fieldC], search: "developer", sort: desc } }) { id } }"#),
      @r###"{"data":{"findManyTestModel":[{"id":1},{"id":3},{"id":2}]}}"###
    );

    insta::assert_snapshot!(
      run_query!(&runner, r#"{ findManyTestModel(orderBy: { _relevance: { fields: [fieldB, fieldC], search: "developer", sort: asc } }) { id } }"#),
      @r###"{"data":{"findManyTestModel":[{"id":2},{"id":3},{"id":1}]}}"###
    );

    Ok(())
}

async fn many_order_by_stmts(runner: Runner) -> TestResult<()> {
    create_test_data(&runner).await?;

    // ID   fieldA                            fieldB                 fieldC        relevance_fieldA   relevance_field_B
    // 1, "developer",              "developer developer developer", NULL             1                     3
    // 2, "developer developer",    "developer",                     NULL             2                     1
    // 3, "a developer",            "developer",                     "developer"      1                     1
    // ORDER BY RELEVANCE fieldA DESC
    // (2, 1, 3)
    // ORDER BY RELEVANCE fieldB DESC
    // (2, 1, 3)
    insta::assert_snapshot!(
      run_query!(&runner, r#"{ findManyTestModel(orderBy: [
              { _relevance: { fields: fieldA, search: "developer", sort: desc } },
              { _relevance: { fields: fieldB, search: "developer", sort: desc } },
            ]) {
              id
            }
          }"#),
      @r###"{"data":{"findManyTestModel":[{"id":2},{"id":1},{"id":3}]}}"###
    );

    // ID   fieldA                            fieldB                 fieldC        relevance_fieldA   relevance_field_B
    // 1, "developer",              "developer developer developer", NULL             1                     3
    // 2, "developer developer",    "developer",                     NULL             2                     1
    // 3, "a developer",            "developer",                     "developer"      1                     1
    // ORDER BY RELEVANCE fieldA ASC
    // (1, 3, 2)
    // ORDER BY RELEVANCE fieldB ASC
    // (3, 1, 2)
    insta::assert_snapshot!(
      run_query!(&runner, r#"{ findManyTestModel(orderBy: [
              { _relevance: { fields: fieldA, search: "developer", sort: asc } },
              { _relevance: { fields: fieldB, search: "developer", sort: asc } },
            ]) {
              id
            }
          }"#),
      @r###"{"data":{"findManyTestModel":[{"id":3},{"id":1},{"id":2}]}}"###
    );

    // ID   fieldA                            fieldB                 fieldC        relevance_fieldA   relevance_field_B
    // 1, "developer",              "developer developer developer", NULL             1                     3
    // 2, "developer developer",    "developer",                     NULL             2                     1
    // 3, "a developer",            "developer",                     "developer"      1                     1
    // ORDER BY RELEVANCE fieldA ASC
    // (1, 3, 2)
    // ORDER BY RELEVANCE fieldB DESC
    // (1, 3, 2)
    insta::assert_snapshot!(
      run_query!(&runner, r#"{ findManyTestModel(orderBy: [
              { _relevance: { fields: fieldA, search: "developer", sort: asc } },
              { _relevance: { fields: fieldB, search: "developer", sort: desc } },
            ]) {
              id
            }
          }"#),
      @r###"{"data":{"findManyTestModel":[{"id":1},{"id":3},{"id":2}]}}"###
    );

    // ID   fieldA                            fieldB                 fieldC        relevance_fieldA   relevance_field_B
    // 1, "developer",              "developer developer developer", NULL             1                     3
    // 2, "developer developer",    "developer",                     NULL             2                     1
    // 3, "a developer",            "developer",                     "developer"      1                     1
    // ORDER BY RELEVANCE fieldA DESC
    // (2, 1, 3)
    // ORDER BY RELEVANCE fieldB ASC
    // (2, 3, 1)
    insta::assert_snapshot!(
      run_query!(&runner, r#"{ findManyTestModel(orderBy: [
              { _relevance: { fields: fieldA, search: "developer", sort: desc } },
              { _relevance: { fields: fieldB, search: "developer", sort: asc } },
            ]) {
              id
            }
          }"#),
      @r###"{"data":{"findManyTestModel":[{"id":2},{"id":3},{"id":1}]}}"###
    );

    Ok(())
}

async fn on_single_field_with_pagination(runner: Runner) -> TestResult<()> {
    create_test_data(&runner).await?;

    // On required field desc
    // ID   fieldA                    relevance
    // 1, "developer",                     1
    // 2, "developer developer"            2
    // 3, "a developer",                   1
    // ORDER BY RELEVANCE fieldA DESC
    // (2, 1, 3)
    // CURSOR on 1, skip 1, take 1
    // (3)
    // insta::assert_snapshot!(
    //   run_query!(&runner, r#"{ findManyTestModel(
    //     cursor: { id: 1 },
    //     skip: 1,
    //     take: 1,
    //     orderBy: { _relevance: { fields: fieldA, search: "developer", sort: desc } }
    //   ) { id } }"#),
    //   @r###"{"data":{"findManyTestModel":[{"id":3}]}}"###
    // );

    match_connector_result!(
      &runner,
      r#"{ findManyTestModel(
            cursor: { id: 1 },
            skip: 1,
            take: 1,
            orderBy: { _relevance: { fields: fieldA, search: "developer", sort: desc } }
          ) { id } }"#,
      MySql(_) => vec![r#"{"data":{"findManyTestModel":[{"id":3}]}}"#, r#"{"data":{"findManyTestModel":[]}}"#],
      _ => vec![r#"{"data":{"findManyTestModel":[{"id":3}]}}"#]
    );

    // On required field asc
    // ID   fieldA                    relevance
    // 1, "developer",                     1
    // 2, "developer developer"            2
    // 3, "a developer",                   1
    // ORDER BY RELEVANCE fieldA ASC
    // (1, 3, 2)
    // CURSOR on 1, skip 1, take 1
    // (3)
    match_connector_result!(
      &runner,
      r#"{ findManyTestModel(
              cursor: { id: 1 },
              skip: 1,
              take: 1,
              orderBy: { _relevance: { fields: fieldA, search: "developer", sort: asc } }
            ) { id } }"#,
      MySql(_) => vec![r#"{"data":{"findManyTestModel":[{"id":3}]}}"#, r#"{"data":{"findManyTestModel":[{"id":2}]}}"#],
      _ => vec![r#"{"data":{"findManyTestModel":[{"id":3}]}}"#]
    );

    Ok(())
}

async fn on_single_nullable_field_with_pagination(runner: Runner) -> TestResult<()> {
    create_test_data(&runner).await?;

    // On nullable field desc
    // ID   fieldC          relevance
    // 1, ""                    0
    // 2, ""                    0
    // 3, "a developer"        1
    // ORDER BY RELEVANCE fieldC DESC
    // (3, 1, 2)
    // CURSOR on 1, skip 1, take 1
    // (2)
    match_connector_result!(
        &runner,
        r#"{ findManyTestModel(
            cursor: { id: 1 },
            skip: 1,
            take: 1,
            orderBy: { _relevance: { fields: fieldC, search: "developer", sort: desc } }
          ) { id } }"#,
      MySql(_) => vec![r#"{"data":{"findManyTestModel":[{"id":2}]}}"#, r#"{"data":{"findManyTestModel":[]}}"#],
      _ => vec![r#"{"data":{"findManyTestModel":[{"id":2}]}}"#]
    );

    // On nullable field asc
    // ID   fieldC          relevance
    // 1, ""                    0
    // 2, ""                    0
    // 3, "a developer"         1
    // ORDER BY RELEVANCE fieldC DESC
    // (1, 2, 3)
    // CURSOR on 1, skip 1
    // (2)
    match_connector_result!(
      &runner,
      r#"{ findManyTestModel(
            cursor: { id: 1 },
            skip: 1,
            orderBy: { _relevance: { fields: fieldC, search: "developer", sort: asc } }
          ) { id } }"#,
      MySql(_) => vec![r#"{"data":{"findManyTestModel":[{"id":2},{"id":3}]}}"#, r#"{"data":{"findManyTestModel":[{"id":3}]}}"#],
      _ => vec![r#"{"data":{"findManyTestModel":[{"id":2},{"id":3}]}}"#]
    );

    Ok(())
}

async fn on_many_fields_with_pagination(runner: Runner) -> TestResult<()> {
    create_test_data(&runner).await?;

    // On required field
    // ID   fieldA                            fieldB                 fieldC        relevance
    // 1, "developer",              "developer developer developer", NULL            4
    // 2, "developer developer",    "developer",                     NULL            3
    // 3, "a developer",            "developer",                     "developer"     2
    // ORDER BY RELEVANCE on [fieldA, fieldB]
    // (1, 2, 3)
    // CURSOR on 2, skip 1, take 1
    // (3)
    insta::assert_snapshot!(
      run_query!(&runner, r#"{ findManyTestModel(
            cursor: { id: 2 },
            skip: 1,
            take: 1,
            orderBy: { _relevance: { fields: [fieldA, fieldB], search: "developer", sort: desc } }
          ) { id } }"#),
      @r###"{"data":{"findManyTestModel":[{"id":3}]}}"###
    );

    // On nullable field
    // ID   fieldA                            fieldB                 fieldC        relevance
    // 1, "developer",              "developer developer developer", NULL             3
    // 2, "developer developer",    "developer",                     NULL             1
    // 3, "a developer",            "developer",                     "developer"      2
    // ORDER BY RELEVANCE on [fieldB, fieldC] DESC
    // (1, 3, 2)
    // CURSOR on 3, skip 1, take 1
    // (2)
    insta::assert_snapshot!(
      run_query!(&runner, r#"{ findManyTestModel(
            cursor: { id: 3 },
            skip: 1,
            take: 1,
            orderBy: { _relevance: { fields: [fieldB, fieldC], search: "developer", sort: desc } }
          ) { id } }"#),
      @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
    );

    // On required field with pagination & order-by scalar
    // ID   fieldA                            fieldB                 fieldC         relevance
    // 1, "developer",              "developer developer developer", NULL               4
    // 2, "developer developer",    "developer",                     NULL               3
    // 3, "a developer",            "developer",                     "developer"        2
    // ORDER BY RELEVANCE on [fieldA, fieldB] DESC
    // (1, 2, 3)
    // ORDER BY fieldA asc
    // (3, 1, 2)
    // CURSOR on 1, skip 1
    // (2)
    insta::assert_snapshot!(
      run_query!(&runner, r#"{ findManyTestModel(
            cursor: { id: 1 },
            skip: 1,
            orderBy: [
              { _relevance: { fields: [fieldA, fieldB], search: "developer", sort: desc } },
              { fieldA: asc }
            ]
          ) { id } }"#),
      @r###"{"data":{"findManyTestModel":[{"id":2},{"id":3}]}}"###
    );

    Ok(())
}

async fn on_many_fields_with_aggr_and_pagination(runner: Runner) -> TestResult<()> {
    create_test_data(&runner).await?;
    create_row(
          &runner,
          r#"{ id: 4, fieldA: "a developer", fieldB: "developer", fieldC: "developer", relations: { create: [{id: 3}] }}"#,
        )
        .await?;

    // On required fields with pagination & order-by aggregation
    // ID   fieldA                            fieldB                 fieldC        relations_count
    // 1, "developer",              "developer developer developer", NULL          0
    // 2, "developer developer",    "developer",                     NULL          0
    // 3, "a developer", "developer",                                "developer"   2
    // 4, "a developer",            "developer",                     "developer"   1
    // ORDER BY RELEVANCE on [fieldA, fieldB]
    // (1, 2, 3, 4)
    // ORDER BY COUNT of Relations ASC
    // (1, 2, 4, 3)
    // CURSOR on 2, skip 1, take 2
    // (4, 3)
    insta::assert_snapshot!(
      run_query!(&runner, r#"{ findManyTestModel(
            cursor: { id: 2 },
            skip: 1,
            take: 2,
            orderBy: [
              { _relevance: { fields: [fieldA, fieldB], search: "developer", sort: desc } },
              { relations: { _count: asc } }
            ]
          ) { id } }"#),
      @r###"{"data":{"findManyTestModel":[{"id":4},{"id":3}]}}"###
    );

    Ok(())
}

async fn on_1m_relation_field(runner: Runner) -> TestResult<()> {
    create_row(
        &runner,
        r#"{ id: 1, fieldA: "developer", fieldB: "developer developer developer", relations: { create: [{ id: 1 }] }}"#,
    )
    .await?;
    create_row(
        &runner,
        r#"{ id: 2, fieldA: "developer developer", fieldB: "developer", relations: { create: [{ id: 2 }] }}"#,
    )
    .await?;
    create_row(
        &runner,
        r#"{ id: 3, fieldA: "a developer", fieldB: "developer", fieldC: "developer", relations: { create: [{ id: 3 }] }}"#,
    )
    .await?;

    // Single field required
    insta::assert_snapshot!(
      run_query!(&runner, r#"{ findManyRelation(orderBy: [{ testModel: { _relevance: { fields: fieldA, search: "developer", sort: desc } } }, { id: desc }]) { id } }"#),
      @r###"{"data":{"findManyRelation":[{"id":2},{"id":3},{"id":1}]}}"###
    );
    insta::assert_snapshot!(
      run_query!(&runner, r#"{ findManyRelation(orderBy: [{ testModel: { _relevance: { fields: fieldA, search: "developer", sort: asc } } }, { id: asc }]) { id } }"#),
      @r###"{"data":{"findManyRelation":[{"id":1},{"id":3},{"id":2}]}}"###
    );

    // Single field optional
    insta::assert_snapshot!(
      run_query!(&runner, r#"{ findManyRelation(orderBy: [{ testModel: { _relevance: { fields: fieldC, search: "developer", sort: desc } } }, { id: desc }]) { id } }"#),
      @r###"{"data":{"findManyRelation":[{"id":3},{"id":2},{"id":1}]}}"###
    );
    insta::assert_snapshot!(
      run_query!(&runner, r#"{ findManyRelation(orderBy: [{ testModel: { _relevance: { fields: fieldC, search: "developer", sort: asc } } }, { id: asc }]) { id } }"#),
      @r###"{"data":{"findManyRelation":[{"id":1},{"id":2},{"id":3}]}}"###
    );

    // Many fields required
    insta::assert_snapshot!(
      run_query!(&runner, r#"{ findManyRelation(orderBy: [{ testModel: { _relevance: { fields: [fieldA, fieldB], search: "developer", sort: desc } } }, { id: desc }]) { id } }"#),
      @r###"{"data":{"findManyRelation":[{"id":1},{"id":2},{"id":3}]}}"###
    );
    insta::assert_snapshot!(
      run_query!(&runner, r#"{ findManyRelation(orderBy: [{ testModel: { _relevance: { fields: [fieldA, fieldB], search: "developer", sort: asc } } }, { id: asc }]) { id } }"#),
      @r###"{"data":{"findManyRelation":[{"id":3},{"id":2},{"id":1}]}}"###
    );

    // Many fields optional
    insta::assert_snapshot!(
      run_query!(&runner, r#"{ findManyRelation(orderBy: [{ testModel: { _relevance: { fields: [fieldB, fieldC], search: "developer", sort: desc } } }, { id: desc }]) { id } }"#),
      @r###"{"data":{"findManyRelation":[{"id":1},{"id":3},{"id":2}]}}"###
    );
    insta::assert_snapshot!(
      run_query!(&runner, r#"{ findManyRelation(orderBy: [{ testModel: { _relevance: { fields: [fieldB, fieldC], search: "developer", sort: asc } } }, { id: asc }]) { id } }"#),
      @r###"{"data":{"findManyRelation":[{"id":2},{"id":3},{"id":1}]}}"###
    );

    // Many fields optional with cursor
    insta::assert_snapshot!(
      run_query!(&runner, r#"{
        findManyRelation(
          orderBy: {
            testModel: { _relevance: { fields: [fieldB, fieldC], search: "developer", sort: desc } }
          }
          cursor: { id: 3 },
          skip: 1
        ) { id } }
      "#),
      @r###"{"data":{"findManyRelation":[{"id":2}]}}"###
    );
    insta::assert_snapshot!(
      run_query!(&runner, r#"{
        findManyRelation(
          orderBy: {
            testModel: { _relevance: { fields: [fieldB, fieldC], search: "developer", sort: asc } }
          }
          cursor: { id: 3 },
          skip: 1
        ) { id } }
      "#),
      @r###"{"data":{"findManyRelation":[{"id":1}]}}"###
    );

    Ok(())
}

async fn create_test_data(runner: &Runner) -> TestResult<()> {
    create_row(
        runner,
        r#"{ id: 1, fieldA: "developer", fieldB: "developer developer developer"}"#,
    )
    .await?;
    create_row(
        runner,
        r#"{ id: 2, fieldA: "developer developer", fieldB: "developer"}"#,
    )
    .await?;
    create_row(
            runner,
            r#"{ id: 3, fieldA: "a developer", fieldB: "developer", fieldC: "developer", relations: { create: [{id: 1}, { id: 2}] }}"#,
        )
        .await?;

    Ok(())
}

async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
    runner
        .query(format!("mutation {{ createOneTestModel(data: {data}) {{ id }} }}"))
        .await?
        .assert_success();
    Ok(())
}

#[test_suite(schema(schema), capabilities(FullTextSearchWithoutIndex))]
mod order_by_relevance_without_index {
    use indoc::indoc;

    fn schema() -> String {
        let schema = indoc! {
            r#"
              model TestModel {
                #id(id, Int, @id)
                fieldA    String
                fieldB    String
                fieldC    String?
                relations Relation[]
              }

              model Relation {
                #id(id, Int, @id)
                testModel   TestModel? @relation(fields: [testModelId], references: [id])
                testModelId Int?
              }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn on_unknown_field(runner: Runner) -> TestResult<()> {
        super::on_unknown_field(runner).await
    }

    #[connector_test]
    async fn on_single_field(runner: Runner) -> TestResult<()> {
        super::on_single_field(runner).await
    }

    #[connector_test]
    async fn on_single_nullable_field(runner: Runner) -> TestResult<()> {
        super::on_single_nullable_field(runner).await
    }

    #[connector_test]
    async fn on_many_fields(runner: Runner) -> TestResult<()> {
        super::on_many_fields(runner).await
    }

    #[connector_test]
    async fn on_many_fields_some_nullable(runner: Runner) -> TestResult<()> {
        super::on_many_fields_some_nullable(runner).await
    }

    #[connector_test]
    async fn many_order_by_stmts(runner: Runner) -> TestResult<()> {
        super::many_order_by_stmts(runner).await
    }

    #[connector_test]
    async fn on_single_field_with_page(runner: Runner) -> TestResult<()> {
        super::on_single_field_with_pagination(runner).await
    }

    #[connector_test]
    async fn on_single_nullable_field_page(runner: Runner) -> TestResult<()> {
        super::on_single_nullable_field_with_pagination(runner).await
    }

    #[connector_test]
    async fn on_many_fields_with_pagination(runner: Runner) -> TestResult<()> {
        super::on_many_fields_with_pagination(runner).await
    }

    #[connector_test]
    async fn on_many_fields_aggr_pagination(runner: Runner) -> TestResult<()> {
        super::on_many_fields_with_aggr_and_pagination(runner).await
    }

    #[connector_test]
    async fn on_1m_relation_field(runner: Runner) -> TestResult<()> {
        super::on_1m_relation_field(runner).await
    }
}

#[test_suite(schema(schema), capabilities(FullTextSearchWithIndex))]
mod order_by_relevance_with_index {
    use indoc::indoc;

    fn schema() -> String {
        let schema = indoc! {
            r#"
              model TestModel {
                #id(id, Int, @id)
                fieldA    String
                fieldB    String
                fieldC    String?
                relations Relation[]
                @@fulltext([fieldA])
                @@fulltext([fieldB])
                @@fulltext([fieldC])
                @@fulltext([fieldA, fieldB])
                @@fulltext([fieldB, fieldC])
              }

              model Relation {
                #id(id, Int, @id)
                testModel   TestModel? @relation(fields: [testModelId], references: [id])
                testModelId Int?
              }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn on_unknown_field(runner: Runner) -> TestResult<()> {
        super::on_unknown_field(runner).await
    }

    #[connector_test]
    async fn on_single_field(runner: Runner) -> TestResult<()> {
        super::on_single_field(runner).await
    }

    #[connector_test]
    async fn on_single_nullable_field(runner: Runner) -> TestResult<()> {
        super::on_single_nullable_field(runner).await
    }

    #[connector_test]
    async fn on_many_fields(runner: Runner) -> TestResult<()> {
        super::on_many_fields(runner).await
    }

    #[connector_test]
    async fn on_many_fields_some_nullable(runner: Runner) -> TestResult<()> {
        super::on_many_fields_some_nullable(runner).await
    }

    #[connector_test]
    async fn many_order_by_stmts(runner: Runner) -> TestResult<()> {
        super::many_order_by_stmts(runner).await
    }

    #[connector_test]
    async fn on_single_field_with_pagination(runner: Runner) -> TestResult<()> {
        super::on_single_field_with_pagination(runner).await
    }

    #[connector_test]
    async fn on_single_nullable_with_pagination(runner: Runner) -> TestResult<()> {
        super::on_single_nullable_field_with_pagination(runner).await
    }

    #[connector_test]
    async fn on_many_fields_with_pagination(runner: Runner) -> TestResult<()> {
        super::on_many_fields_with_pagination(runner).await
    }

    #[connector_test]
    async fn on_many_fields_aggr_pagination(runner: Runner) -> TestResult<()> {
        super::on_many_fields_with_aggr_and_pagination(runner).await
    }

    #[connector_test]
    async fn on_1m_relation_field(runner: Runner) -> TestResult<()> {
        super::on_1m_relation_field(runner).await
    }
}
