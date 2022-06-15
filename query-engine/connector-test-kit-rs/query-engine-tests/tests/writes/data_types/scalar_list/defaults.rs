use query_engine_tests::*;

// General defaults tests
#[test_suite(schema(common), capabilities(ScalarLists))]
mod basic {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn common() -> String {
        let schema = indoc! {
            r#"model ScalarModel {
              #id(id, Int, @id)
              strings   String[]   @default(["Foo", "Bar", "Baz"])
              ints      Int[]      @default([1, 2, 3])
              floats    Float[]    @default([1.1, 2.2, 3.3])
              booleans  Boolean[]  @default([true, false, false, true])
              enums     MyEnum[]   @default([A, B, B, A])
              dateTimes DateTime[] @default(["2019-07-31T23:59:01.000Z", "2012-07-31T23:59:01.000Z"])
              bytes     Bytes[]    @default(["dGVzdA==", "dA=="])
            }

            enum MyEnum {
              A
              B
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn basic_write(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
          createOneScalarModel(data: {
            id: 1
          }) {
            strings
            ints
            floats
            booleans
            enums
            dateTimes
            bytes
          }
        }"#),
          @r###"{"data":{"createOneScalarModel":{"strings":["Foo","Bar","Baz"],"ints":[1,2,3],"floats":[1.1,2.2,3.3],"booleans":[true,false,false,true],"enums":["A","B","B","A"],"dateTimes":["2019-07-31T23:59:01.000Z","2012-07-31T23:59:01.000Z"],"bytes":["dGVzdA==","dA=="]}}}"###
        );

        Ok(())
    }

    fn common_empty() -> String {
        let schema = indoc! {
            r#"model ScalarModel {
              #id(id, Int, @id)
              strings   String[]   @default([])
              ints      Int[]      @default([])
              floats    Float[]    @default([])
              booleans  Boolean[]  @default([])
              enums     MyEnum[]   @default([])
              dateTimes DateTime[] @default([])
              bytes     Bytes[]    @default([])
            }

            enum MyEnum {
              A
              B
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(common_empty))]
    async fn basic_empty_write(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
          createOneScalarModel(data: {
            id: 1
          }) {
            strings
            ints
            floats
            booleans
            enums
            dateTimes
            bytes
          }
        }"#),
          @r###"{"data":{"createOneScalarModel":{"strings":[],"ints":[],"floats":[],"booleans":[],"enums":[],"dateTimes":[],"bytes":[]}}}"###
        );

        Ok(())
    }
}

#[test_suite(schema(common), capabilities(ScalarLists, DecimalType))]
mod decimal {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn common() -> String {
        let schema = indoc! {
            r#"
                model ScalarModel {
                    #id(id, Int, @id)
                    decimals  Decimal[] @default(["123.321", "9999.9999"])
                    decimals2 Decimal[] @default([123.321, 9999.9999])
                }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn basic_write(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
          createOneScalarModel(data: {
            id: 1
          }) {
            decimals
            decimals2
          }
        }"#),
          @r###"{"data":{"createOneScalarModel":{"decimals":["123.321","9999.9999"],"decimals2":["123.321","9999.9999"]}}}"###
        );

        Ok(())
    }

    fn common_empty() -> String {
        let schema = indoc! {
            r#"model ScalarModel {
              #id(id, Int, @id)
              decimals Decimal[] @default([])
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(common_empty))]
    async fn basic_empty_write(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
          createOneScalarModel(data: {
            id: 1
          }) {
            decimals
          }
        }"#),
          @r###"{"data":{"createOneScalarModel":{"decimals":[]}}}"###
        );

        Ok(())
    }
}

#[test_suite(schema(common), capabilities(ScalarLists, Json, JsonLists))]
mod json {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn common() -> String {
        let schema = indoc! {
            r#"
                model ScalarModel {
                    #id(id, Int, @id)
                    jsons Json[] @default(["{ \"a\": [\"b\"] }", "3"])
                }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn basic_write(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
          createOneScalarModel(data: {
            id: 1
          }) {
            jsons
          }
        }"#),
          @r###"{"data":{"createOneScalarModel":{"jsons":["{\"a\":[\"b\"]}","3"]}}}"###
        );

        Ok(())
    }

    fn common_empty() -> String {
        let schema = indoc! {
            r#"model ScalarModel {
              #id(id, Int, @id)
              jsons Json[] @default([])
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(common_empty))]
    async fn basic_empty_write(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
          createOneScalarModel(data: {
            id: 1
          }) {
            jsons
          }
        }"#),
          @r###"{"data":{"createOneScalarModel":{"jsons":[]}}}"###
        );

        Ok(())
    }
}
