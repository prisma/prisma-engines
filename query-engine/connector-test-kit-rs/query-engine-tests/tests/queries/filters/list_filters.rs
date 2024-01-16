use query_engine_tests::*;

#[test_suite(schema(common_list_types), capabilities(ScalarLists))]
mod lists {
    use indoc::indoc;
    use query_engine_tests::run_query;

    #[connector_test]
    async fn equality(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // string equals
        insta::assert_snapshot!(
          list_query(&runner, "string", "equals", r#"["a", "A", "c"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        // string NOT equals
        insta::assert_snapshot!(
          not_list_query(&runner, "string", "equals", r#"["a", "A", "c"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // int equals
        insta::assert_snapshot!(
          list_query(&runner, "int", "equals", r#"[1, 2, 3]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        // int NOT equals
        insta::assert_snapshot!(
          not_list_query(&runner, "int", "equals", r#"[1, 2, 3]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // float equals
        insta::assert_snapshot!(
          list_query(&runner, "float", "equals", r#"[1.1, 2.2, 3.3]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        // float NOT equals
        insta::assert_snapshot!(
          not_list_query(&runner, "float", "equals", r#"[1.1, 2.2, 3.3]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // bInt equals
        insta::assert_snapshot!(
          list_query(&runner, "bInt", "equals", r#"["100", "200", "300"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        // bInt NOT equals
        insta::assert_snapshot!(
          not_list_query(&runner, "bInt", "equals", r#"["100", "200", "300"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // bool equals
        insta::assert_snapshot!(
          list_query(&runner, "bool", "equals", r#"[true]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        // bool NOT equals
        insta::assert_snapshot!(
          not_list_query(&runner, "bool", "equals", r#"[true]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // bytes equals
        insta::assert_snapshot!(
          list_query(&runner, "bytes", "equals", r#"["dGVzdA==", "dA=="]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        // bytes NOT equals
        insta::assert_snapshot!(
          not_list_query(&runner, "bytes", "equals", r#"["dGVzdA==", "dA=="]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // datetime equals
        insta::assert_snapshot!(
          list_query(&runner, "dt", "equals", r#"["1969-01-01T10:33:59.000Z", "2018-12-05T12:34:23.000Z"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        // datetime NOT equals
        insta::assert_snapshot!(
          not_list_query(&runner, "dt", "equals", r#"["1969-01-01T10:33:59.000Z", "2018-12-05T12:34:23.000Z"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn has(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // has string
        insta::assert_snapshot!(
          list_query(&runner, "string", "has", r#""A""#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        // has NOT string
        insta::assert_snapshot!(
          not_list_query(&runner, "string", "has", r#""A""#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // has int
        insta::assert_snapshot!(
          list_query(&runner, "int", "has", "2").await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        // has NOT int
        insta::assert_snapshot!(
          not_list_query(&runner, "int", "has", "2").await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // has float
        insta::assert_snapshot!(
          list_query(&runner, "float", "has", "1.1").await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        // has NOT float
        insta::assert_snapshot!(
          not_list_query(&runner, "float", "has", "1.1").await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // has bInt
        insta::assert_snapshot!(
          list_query(&runner, "bInt", "has", r#""200""#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        // has NOT bInt
        insta::assert_snapshot!(
          not_list_query(&runner, "bInt", "has", r#""200""#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // has datetime
        insta::assert_snapshot!(
          list_query(&runner, "dt", "has", r#""2018-12-05T12:34:23.000Z""#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        // has NOT datetime
        insta::assert_snapshot!(
          not_list_query(&runner, "dt", "has", r#""2018-12-05T12:34:23.000Z""#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // has boolean
        insta::assert_snapshot!(
        list_query(&runner, "bool", "has", "true").await?,
         @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        // has NOT boolean
        insta::assert_snapshot!(
        not_list_query(&runner, "bool", "has", "true").await?,
         @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // has bytes
        insta::assert_snapshot!(
            list_query(&runner, "bytes", "has", r#""dGVzdA==""#).await?,
            @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        // has NOT bytes
        insta::assert_snapshot!(
            not_list_query(&runner, "bytes", "has", r#""dGVzdA==""#).await?,
            @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn has_some(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // string hasSome
        insta::assert_snapshot!(
          list_query(&runner, "string", "hasSome", r#"["A", "c"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "string", "hasSome", r#"[]"#).await?,
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        // string NOT hasSome
        insta::assert_snapshot!(
          not_list_query(&runner, "string", "hasSome", r#"["A", "c"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "string", "hasSome", r#"[]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // int hasSome
        insta::assert_snapshot!(
          list_query(&runner, "int", "hasSome", r#"[2, 10]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "int", "hasSome", r#"[]"#).await?,
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        // int NOT hasSome
        insta::assert_snapshot!(
          not_list_query(&runner, "int", "hasSome", r#"[2, 10]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "int", "hasSome", r#"[]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // float hasSome
        insta::assert_snapshot!(
          list_query(&runner, "float", "hasSome", r#"[1.1, 5.5]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "float", "hasSome", r#"[]"#).await?,
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        // float NOT hasSome
        insta::assert_snapshot!(
          not_list_query(&runner, "float", "hasSome", r#"[1.1, 5.5]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "float", "hasSome", r#"[]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // bInt hasSome
        insta::assert_snapshot!(
          list_query(&runner, "bInt", "hasSome", r#"["200", "5000"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "bInt", "hasSome", r#"[]"#).await?,
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        // bInt NOT hasSome
        insta::assert_snapshot!(
          not_list_query(&runner, "bInt", "hasSome", r#"["200", "5000"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "bInt", "hasSome", r#"[]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // bool hasSome
        insta::assert_snapshot!(
          list_query(&runner, "bool", "hasSome", r#"[true, false]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "bool", "hasSome", r#"[]"#).await?,
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        // bool NOT hasSome
        insta::assert_snapshot!(
          not_list_query(&runner, "bool", "hasSome", r#"[true, false]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "bool", "hasSome", r#"[]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // dt hasSome
        insta::assert_snapshot!(
          list_query(
              &runner,
              "dt",
              "hasSome",
              r#"["2018-12-05T12:34:23.000Z", "2019-12-05T12:34:23.000Z"]"#,
          )
          .await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        insta::assert_snapshot!(
          list_query(
              &runner,
              "bytes",
              "hasSome",
              r#"["dGVzdA==", "bG9va2luZyBmb3Igc29tZXRoaW5nPw=="]"#,
          )
          .await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // dt NOT hasSome
        insta::assert_snapshot!(
          not_list_query(
              &runner,
              "dt",
              "hasSome",
              r#"["2018-12-05T12:34:23.000Z", "2019-12-05T12:34:23.000Z"]"#,
          )
          .await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(
              &runner,
              "bytes",
              "hasSome",
              r#"["dGVzdA==", "bG9va2luZyBmb3Igc29tZXRoaW5nPw=="]"#,
          )
          .await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn has_every(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // string hasEvery
        insta::assert_snapshot!(
          list_query(&runner, "string", "hasEvery", r#"["A", "d"]"#).await?,
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "string", "hasEvery", r#"["A"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // string NOT hasEvery
        insta::assert_snapshot!(
          not_list_query(&runner, "string", "hasEvery", r#"["A", "d"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "string", "hasEvery", r#"["A"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // int hasEvery
        insta::assert_snapshot!(
          list_query(&runner, "int", "hasEvery", r#"[2, 10]"#).await?,
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "int", "hasEvery", r#"[2]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // int NOT hasEvery
        insta::assert_snapshot!(
          not_list_query(&runner, "int", "hasEvery", r#"[2, 10]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "int", "hasEvery", r#"[2]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // float hasEvery
        insta::assert_snapshot!(
          list_query(&runner, "float", "hasEvery", r#"[1.1, 5.5]"#).await?,
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "float", "hasEvery", r#"[1.1]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // float NOT hasEvery
        insta::assert_snapshot!(
          not_list_query(&runner, "float", "hasEvery", r#"[1.1, 5.5]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "float", "hasEvery", r#"[1.1]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // bInt hasEvery
        insta::assert_snapshot!(
          list_query(&runner, "bInt", "hasEvery", r#"["200", "5000"]"#).await?,
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "bInt", "hasEvery", r#"["200"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // bInt NOT hasEvery
        insta::assert_snapshot!(
          not_list_query(&runner, "bInt", "hasEvery", r#"["200", "5000"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "bInt", "hasEvery", r#"["200"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // dt hasEvery
        insta::assert_snapshot!(
          list_query(&runner, "dt", "hasEvery", r#"["2018-12-05T12:34:23.000Z"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        insta::assert_snapshot!(
          list_query(
            &runner,
            "dt",
            "hasEvery",
            r#"["2018-12-05T12:34:23.000Z", "2019-12-05T12:34:23.000Z"]"#,
          )
          .await?,
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        // dt NOT hasEvery
        insta::assert_snapshot!(
          not_list_query(&runner, "dt", "hasEvery", r#"["2018-12-05T12:34:23.000Z"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(
            &runner,
            "dt",
            "hasEvery",
            r#"["2018-12-05T12:34:23.000Z", "2019-12-05T12:34:23.000Z"]"#,
          )
          .await?,
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // bool hasEvery
        insta::assert_snapshot!(
          list_query(&runner, "bool", "hasEvery", r#"[true, false]"#).await?,
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "bool", "hasEvery", r#"[true]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // bool NOT hasEvery
        insta::assert_snapshot!(
          not_list_query(&runner, "bool", "hasEvery", r#"[true, false]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "bool", "hasEvery", r#"[true]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // bytes hasEvery
        insta::assert_snapshot!(
          list_query(&runner, "bytes", "hasEvery", r#"["dGVzdA=="]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        insta::assert_snapshot!(
          list_query(
            &runner,
            "bytes",
            "hasEvery",
            r#"["dGVzdA==", "bG9va2luZyBmb3Igc29tZXRoaW5nPw=="]"#,
          )
          .await?,
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        // bytes NOT hasEvery
        insta::assert_snapshot!(
          not_list_query(&runner, "bytes", "hasEvery", r#"["dGVzdA=="]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(
            &runner,
            "bytes",
            "hasEvery",
            r#"["dGVzdA==", "bG9va2luZyBmb3Igc29tZXRoaW5nPw=="]"#,
          )
          .await?,
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn is_empty(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // string isEmpty
        insta::assert_snapshot!(
          list_query(&runner, "string", "isEmpty", "true").await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "string", "isEmpty", "false").await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // string NOT isEmpty
        insta::assert_snapshot!(
          not_list_query(&runner, "string", "isEmpty", "true").await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "string", "isEmpty", "false").await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // int isEmpty
        insta::assert_snapshot!(
          list_query(&runner, "int", "isEmpty", "true").await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "int", "isEmpty", "false").await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // int NOT isEmpty
        insta::assert_snapshot!(
          not_list_query(&runner, "int", "isEmpty", "true").await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "int", "isEmpty", "false").await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // float isEmpty
        insta::assert_snapshot!(
          list_query(&runner, "float", "isEmpty", "true").await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "float", "isEmpty", "false").await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // float NOT isEmpty
        insta::assert_snapshot!(
          not_list_query(&runner, "float", "isEmpty", "true").await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "float", "isEmpty", "false").await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // bInt isEmpty
        insta::assert_snapshot!(
          list_query(&runner, "bInt", "isEmpty", "true").await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "bInt", "isEmpty", "false").await?,
        @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // bInt isEmpty
        insta::assert_snapshot!(
          not_list_query(&runner, "bInt", "isEmpty", "true").await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "bInt", "isEmpty", "false").await?,
        @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // dt isEmpty
        insta::assert_snapshot!(
          list_query(&runner, "dt", "isEmpty", "true").await?,
        @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "dt", "isEmpty", "false").await?,
        @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // dt NOT isEmpty
        insta::assert_snapshot!(
          not_list_query(&runner, "dt", "isEmpty", "true").await?,
        @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "dt", "isEmpty", "false").await?,
        @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        // bool isEmpty
        insta::assert_snapshot!(
          list_query(&runner, "bool", "isEmpty", "true").await?,
        @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "bool", "isEmpty", "false").await?,
        @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // bool NOT isEmpty
        insta::assert_snapshot!(
          not_list_query(&runner, "bool", "isEmpty", "true").await?,
        @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "bool", "isEmpty", "false").await?,
        @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    // Cockroachdb does not like the bytes empty array check in v21 but this will be fixed in 22.
    #[connector_test(exclude(CockroachDB))]
    async fn is_empty_bytes(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // isEmpty bytes
        insta::assert_snapshot!(
          list_query(&runner, "bytes", "isEmpty", "true").await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "bytes", "isEmpty", "false").await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // NOT isEmpty bytes
        insta::assert_snapshot!(
          not_list_query(&runner, "bytes", "isEmpty", "true").await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "bytes", "isEmpty", "false").await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn has_every_empty(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { string: { hasEvery: [] }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { NOT: { string: { hasEvery: [] } }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    async fn test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(indoc::indoc! { r#"
              mutation {
                createOneTestModel(data: {
                  id:      1,
                  string:  ["a", "A", "c"],
                  int:     [1, 2, 3],
                  float:   [1.1, 2.2, 3.3],
                  bInt:    ["100", "200", "300"],
                  dt:      ["1969-01-01T10:33:59.000Z", "2018-12-05T12:34:23.000Z"],
                  bool:    [true],
                  bytes:   ["dGVzdA==", "dA=="],
                }) { id }
              }
            "#})
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"
              mutation {
                createOneTestModel(data: {
                  id:      2,
                  string:  [],
                  int:     [],
                  float:   [],
                  bInt:    [],
                  dt:      [],
                  bool:    [],
                  bytes:   []
                }) { id }
            }
            "#})
            .await?
            .assert_success();

        Ok(())
    }
}

#[test_suite(schema(schema), capabilities(ScalarLists, DecimalType))]
mod decimal_lists {
    use indoc::indoc;
    use query_engine_tests::run_query;

    pub fn schema() -> String {
        let schema = indoc! {
            "model TestModel {
                #id(id, Int, @id)
                decimal Decimal[]
            }"
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn equality(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // equals decimal
        insta::assert_snapshot!(
          list_query(&runner, "decimal", "equals", r#"["11.11", "22.22", "33.33"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // NOT equals decimal
        insta::assert_snapshot!(
          not_list_query(&runner, "decimal", "equals", r#"["11.11", "22.22", "33.33"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn has(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // has
        insta::assert_snapshot!(
          list_query(&runner, "decimal", "has", "33.33").await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // NOT has
        insta::assert_snapshot!(
          not_list_query(&runner, "decimal", "has", "33.33").await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn has_some(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // hasSome decimal
        insta::assert_snapshot!(
          list_query(&runner, "decimal", "hasSome", r#"[55.55, 33.33]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // NOT hasSome decimal
        insta::assert_snapshot!(
          not_list_query(&runner, "decimal", "hasSome", r#"[55.55, 33.33]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn has_every(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // hasEvery decimal
        insta::assert_snapshot!(
          list_query(&runner, "decimal", "hasEvery", r#"[55.55, 33.33]"#).await?,
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "decimal", "hasEvery", r#"[33.33]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // NOT hasEvery decimal
        insta::assert_snapshot!(
          not_list_query(&runner, "decimal", "hasEvery", r#"[55.55, 33.33]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "decimal", "hasEvery", r#"[33.33]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn is_empty(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // isEmpty decimal
        insta::assert_snapshot!(
          list_query(&runner, "decimal", "isEmpty", "true").await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "decimal", "isEmpty", "false").await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // NOT isEmpty decimal
        insta::assert_snapshot!(
          not_list_query(&runner, "decimal", "isEmpty", "true").await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "decimal", "isEmpty", "false").await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn has_every_empty(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // hasEvery decimal
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { decimal: { hasEvery: [] }}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // NOT hasEvery decimal
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel(where: { NOT: { decimal: { hasEvery: [] }}}) { id }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    async fn test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(indoc::indoc! { r#"
              mutation {
                createOneTestModel(data: {
                  id:      1,
                  decimal: ["11.11", "22.22", "33.33"],
                }) { id }
              }
            "#})
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"
              mutation {
                createOneTestModel(data: {
                  id:      2,
                  decimal: [],
                }) { id }
            }
            "#})
            .await?
            .assert_success();

        Ok(())
    }
}

// CockroachDB cannot store Json[], but can process them in memory.
// See https://github.com/cockroachdb/cockroach/issues/23468.
#[test_suite(schema(schema), capabilities(ScalarLists, Json), exclude(CockroachDb))]
mod json_lists {
    use indoc::indoc;

    fn schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              json Json[]
            }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn equality(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // equals json
        insta::assert_snapshot!(
          list_query(&runner, "json", "equals", r#"["{}", "{\"int\":5}", "[1, 2, 3]"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "json", "equals", r#"["null", "\"test\""]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":3}]}}"###
        );

        // NOT equals json
        insta::assert_snapshot!(
          not_list_query(&runner, "json", "equals", r#"["{}", "{\"int\":5}", "[1, 2, 3]"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":3}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "json", "equals", r#"["null", "\"test\""]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn has(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // has json
        insta::assert_snapshot!(
          list_query(&runner, "json", "has", r#""[1, 2, 3]""#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "json", "has", r#""null""#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":3}]}}"###
        );

        // NOT has json
        insta::assert_snapshot!(
          not_list_query(&runner, "json", "has", r#""[1, 2, 3]""#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":3}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "json", "has", r#""null""#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn has_some(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // hasSome json
        insta::assert_snapshot!(
          list_query(&runner, "json", "hasSome", r#"["{}", "[1]"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "json", "hasSome", r#"["null", "\"test 2\""]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":3}]}}"###
        );

        // NOT hasSome json
        insta::assert_snapshot!(
          not_list_query(&runner, "json", "hasSome", r#"["{}", "[1]"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":3}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "json", "hasSome", r#"["null", "\"test 2\""]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn has_every(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // hasEvery json
        insta::assert_snapshot!(
          list_query(&runner, "json", "hasEvery", r#"["{}", "[1]"]"#).await?,
          @r###"{"data":{"findManyTestModel":[]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "json", "hasEvery", r#"["{}"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "json", "hasEvery", r#"["null"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":3}]}}"###
        );

        // NOT hasEvery json
        insta::assert_snapshot!(
          not_list_query(&runner, "json", "hasEvery", r#"["{}", "[1]"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "json", "hasEvery", r#"["{}"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2},{"id":3}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "json", "hasEvery", r#"["null"]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn is_empty(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // isEmpty json
        insta::assert_snapshot!(
          list_query(&runner, "json", "isEmpty", "true").await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "json", "isEmpty", "false").await?,
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":3}]}}"###
        );

        // NOT isEmpty json
        insta::assert_snapshot!(
          not_list_query(&runner, "json", "isEmpty", "true").await?,
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":3}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "json", "isEmpty", "false").await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    async fn test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(indoc::indoc! { r#"
              mutation {
                createOneTestModel(data: {
                  id:   1,
                  json: ["{}", "{\"int\":5}", "[1, 2, 3]"]
                }) { id }
              }
            "#})
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"
              mutation {
                createOneTestModel(data: {
                  id:   2,
                  json: []
                }) { id }
            }
            "#})
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"
              mutation {
                createOneTestModel(data: {
                  id:   3,
                  json: ["null", "\"test\""]
                }) { id }
            }
            "#})
            .await?
            .assert_success();

        Ok(())
    }
}

#[test_suite(schema(schema), capabilities(ScalarLists, Enums))]
mod enum_lists {
    use indoc::indoc;

    fn schema() -> String {
        let schema = indoc! {
            r#"
            model TestModel {
              #id(id, Int, @id)
              enum TestEnum[]
            }

            enum TestEnum {
                A
                B
            }
            "#
        };

        schema.to_owned()
    }

    // This will be fixed in v22
    #[connector_test(exclude(CockroachDB))]
    async fn equality(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // equals enum
        insta::assert_snapshot!(
          list_query(&runner, "enum", "equals", r#"[A, B, B, A]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // NOT equals enum
        insta::assert_snapshot!(
          not_list_query(&runner, "enum", "equals", r#"[A, B, B, A]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn has(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // has enum
        insta::assert_snapshot!(
          list_query(&runner, "enum", "has", "A").await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // NOT has enum
        insta::assert_snapshot!(
          not_list_query(&runner, "enum", "has", "A").await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn has_some(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // hasSome enum
        insta::assert_snapshot!(
          list_query(&runner, "enum", "hasSome", r#"[A]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // NOT hasSome enum
        insta::assert_snapshot!(
          not_list_query(&runner, "enum", "hasSome", r#"[A]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn has_every(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // hasEvery enum
        insta::assert_snapshot!(
          list_query(&runner, "enum", "hasEvery", r#"[A, B]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // NOT hasEvery enum
        insta::assert_snapshot!(
          not_list_query(&runner, "enum", "hasEvery", r#"[A, B]"#).await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    // This will be fixed in v22
    #[connector_test(exclude(CockroachDB))]
    async fn is_empty(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // isEmpty enum
        insta::assert_snapshot!(
          list_query(&runner, "enum", "isEmpty", "true").await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );
        insta::assert_snapshot!(
          list_query(&runner, "enum", "isEmpty", "false").await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // NOT isEmpty enum
        insta::assert_snapshot!(
          not_list_query(&runner, "enum", "isEmpty", "true").await?,
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        insta::assert_snapshot!(
          not_list_query(&runner, "enum", "isEmpty", "false").await?,
          @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        Ok(())
    }

    async fn test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(indoc::indoc! { r#"
              mutation {
                createOneTestModel(data: {
                  id:   1,
                  enum: [A, B, B, A]
                }) { id }
              }
            "#})
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"
              mutation {
                createOneTestModel(data: {
                  id:   2,
                  enum: [],
                }) { id }
            }
            "#})
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"
              mutation {
                createOneTestModel(data: {
                  id: 3,
                }) { id }
            }
            "#})
            .await?
            .assert_success();

        Ok(())
    }
}

async fn list_query(runner: &Runner, field: &str, operation: &str, comparator: &str) -> TestResult<String> {
    let res = run_query!(
        runner,
        format!(
            r#"query {{
              findManyTestModel(where: {{
                {field}: {{ {operation}: {comparator} }}
              }}) {{
                id
              }}
            }}
            "#
        )
    );

    Ok(res)
}

async fn not_list_query(runner: &Runner, field: &str, operation: &str, comparator: &str) -> TestResult<String> {
    let res = run_query!(
        runner,
        format!(
            r#"
            query {{
                findManyTestModel(where: {{
                NOT: {{ {field}: {{ {operation}: {comparator} }} }}
                }}) {{
                id
                }}
            }}
            "#
        )
    );

    Ok(res)
}
