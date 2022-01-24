use query_engine_tests::*;

/// Asserts common basics for composite type writes.
#[test_suite(schema(all_composite_types))]
mod common {
    use query_engine_tests::run_query;

    /// Asserts that all required types that are expected to work on composites do indeed work.
    #[connector_test]
    async fn all_required_types_work(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
              createOneTestModel(data: {
                  id: 1,
                  allRequired: {
                    str: "foo"
                    bool: true,
                    int: 123,
                    bInt: "9223372036854775807",
                    float: 1.2345,
                    dt: "1969-01-01T10:33:59.000Z",
                    json: "{\"a\":\"b\"}",
                    bytes: "dGVzdA==",
                    enum: Foo
                  }
              }) {
                  allRequired {
                    str
                    bool
                    int
                    bInt
                    float
                    dt
                    json
                    bytes
                    enum
                  }
              }}"#),
          @r###"{"data":{"createOneTestModel":{"allRequired":{"str":"foo","bool":true,"int":123,"bInt":"9223372036854775807","float":1.2345,"dt":"1969-01-01T10:33:59.000Z","json":"{\"a\":\"b\"}","bytes":"dGVzdA==","enum":"Foo"}}}}"###
        );

        Ok(())
    }

    /// Asserts that all required types that are expected to work on composites do indeed work.
    #[connector_test]
    async fn all_optional_types_work(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
              createOneTestModel(data: {
                  id: 1,
                  allOptional: {
                    str: "foo"
                    bool: true,
                    int: 123,
                    bInt: "9223372036854775807",
                    float: 1.2345,
                    dt: "1969-01-01T10:33:59.000Z",
                    json: "{\"a\":\"b\"}",
                    bytes: "dGVzdA==",
                    enum: Foo
                  }
              }) {
                  allOptional {
                    str
                    bool
                    int
                    bInt
                    float
                    dt
                    json
                    bytes
                    enum
                  }
              }}"#),
          @r###"{"data":{"createOneTestModel":{"allOptional":{"str":"foo","bool":true,"int":123,"bInt":"9223372036854775807","float":1.2345,"dt":"1969-01-01T10:33:59.000Z","json":"{\"a\":\"b\"}","bytes":"dGVzdA==","enum":"Foo"}}}}"###
        );

        // Explicit null set
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
                createOneTestModel(data: {
                    id: 2,
                    allOptional: {
                      str: null
                      bool: null,
                      int: null,
                      bInt: null,
                      float: null,
                      dt: null,
                      json: null,
                      bytes: null,
                      enum: null
                    }
                }) {
                    allOptional {
                      str
                      bool
                      int
                      bInt
                      float
                      dt
                      json
                      bytes
                      enum
                    }
                }}"#),
          @r###"{"data":{"createOneTestModel":{"allOptional":{"str":null,"bool":null,"int":null,"bInt":null,"float":null,"dt":null,"json":null,"bytes":null,"enum":null}}}}"###
        );

        // Set nothing
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
                createOneTestModel(data: {
                    id: 3,
                    allOptional: {}
                }) {
                    allOptional {
                      str
                      bool
                      int
                      bInt
                      float
                      dt
                      json
                      bytes
                      enum
                    }
                }}"#),
          @r###"{"data":{"createOneTestModel":{"allOptional":{"str":null,"bool":null,"int":null,"bInt":null,"float":null,"dt":null,"json":null,"bytes":null,"enum":null}}}}"###
        );

        Ok(())
    }

    /// Asserts that all list types that are expected to work on composites do indeed work.
    #[connector_test]
    async fn all_list_types_work(runner: Runner) -> TestResult<()> {
        // Empty lists
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
                createOneTestModel(data: {
                    id: 1,
                    allLists: {
                      str: [],
                      bool: [],
                      int: [],
                      bInt: [],
                      float: [],
                      dt: [],
                      json: [],
                      bytes: [],
                      enum: []
                    }
                }) {
                    allLists {
                      str
                      bool
                      int
                      bInt
                      float
                      dt
                      json
                      bytes
                      enum
                    }
                }}"#),
          @r###"{"data":{"createOneTestModel":{"allLists":{"str":[],"bool":[],"int":[],"bInt":[],"float":[],"dt":[],"json":[],"bytes":[],"enum":[]}}}}"###
        );

        // Lists with values
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
              createOneTestModel(data: {
                  id: 2,
                  allLists: {
                    str: ["foo"],
                    bool: [true],
                    int: [123],
                    bInt: ["9223372036854775807"],
                    float: [1.2345],
                    dt: ["1969-01-01T10:33:59.000Z"],
                    json: ["{\"a\":\"b\"}"],
                    bytes: ["dGVzdA=="],
                    enum: [Foo]
                  }
              }) {
                  allLists {
                    str
                    bool
                    int
                    bInt
                    float
                    dt
                    json
                    bytes
                    enum
                  }
              }}"#),
          @r###"{"data":{"createOneTestModel":{"allLists":{"str":["foo"],"bool":[true],"int":[123],"bInt":["9223372036854775807"],"float":[1.2345],"dt":["1969-01-01T10:33:59.000Z"],"json":["{\"a\":\"b\"}"],"bytes":["dGVzdA=="],"enum":["Foo"]}}}}"###
        );

        Ok(())
    }
}
