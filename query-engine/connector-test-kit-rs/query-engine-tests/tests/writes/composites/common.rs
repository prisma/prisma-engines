use indoc::indoc;
use query_engine_tests::*;

/// Todo: Requires enums to work.
/// Asserts common basics for composite type writes.
#[test_suite(schema(all_composite_types))]
mod common {
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
                    bInt: 123123123123123123123123123123,
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
              }"#),
          @r###""###
        );

        Ok(())
    }

    /// Asserts that all required types that are expected to work on composites do indeed work.
    /// Todo: Requires enums to work.
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
                    bInt: 123123123123123123123123123123,
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
              }"#),
          @r###""###
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
                }"#),
          @r###""###
        );

        // Explicit null set
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
                createOneTestModel(data: {
                    id: 3,
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
                }"#),
          @r###""###
        );

        // Set nothing
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
                createOneTestModel(data: {
                    id: 4,
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
                }"#),
          @r###""###
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
                }"#),
          @r###""###
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
                    bInt: [123123123123123123123123123123],
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
              }"#),
          @r###""###
        );

        Ok(())
    }
}
