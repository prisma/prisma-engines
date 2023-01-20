use query_engine_tests::*;

#[test_suite(capabilities(CreateMany))]
mod create_many {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query};

    fn schema_1() -> String {
        let schema = indoc! {
            r#"model Test {
              #id(id, Int, @id)
              str1 String
              str2 String?
              str3 String? @default("SOME_DEFAULT")
            }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_1))]
    async fn basic_create_many(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createManyTest(data: [
              { id: 1, str1: "1", str2: "1", str3: "1"},
              { id: 2, str1: "2",            str3: null},
              { id: 3, str1: "1"},
            ]) {
              count
            }
          }"#),
          @r###"{"data":{"createManyTest":{"count":3}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(schema_1))]
    async fn basic_create_many_shorthand(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createManyTest(data: { id: 1, str1: "1", str2: "1", str3: "1"}) {
              count
            }
          }"#),
          @r###"{"data":{"createManyTest":{"count":1}}}"###
        );

        Ok(())
    }

    fn schema_2() -> String {
        let schema = indoc! {
            r#"model Test {
              #id(id, Int, @id @default(autoincrement()))
              str1 String
              str2 String?
              str3 String? @default("SOME_DEFAULT")
            }"#
        };

        schema.to_owned()
    }

    // Covers: AutoIncrement ID working with basic autonincrement functionality.
    #[connector_test(schema(schema_2), capabilities(CreateManyWriteableAutoIncId), exclude(CockroachDb))]
    async fn basic_create_many_autoincrement(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createManyTest(data: [
              { id: 123, str1: "1", str2: "1", str3: "1"},
              { id: 321, str1: "2",            str3: null},
              {          str1: "1"},
            ]) {
              count
            }
          }"#),
          @r###"{"data":{"createManyTest":{"count":3}}}"###
        );

        Ok(())
    }

    fn schema_2_cockroachdb() -> String {
        let schema = indoc! {
            r#"model Test {
              #id(id, BigInt, @id @default(autoincrement()))
              str1 String
              str2 String?
              str3 String? @default("SOME_DEFAULT")
            }"#
        };

        schema.to_owned()
    }

    // Covers: AutoIncrement ID working with basic autonincrement functionality.
    #[connector_test(schema(schema_2_cockroachdb), only(CockroachDb))]
    async fn basic_create_many_autoincrement_cockroachdb(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createManyTest(data: [
              { id: 123, str1: "1", str2: "1", str3: "1"},
              { id: 321, str1: "2",            str3: null},
              {          str1: "1"},
            ]) {
              count
            }
          }"#),
          @r###"{"data":{"createManyTest":{"count":3}}}"###
        );

        Ok(())
    }

    fn schema_3() -> String {
        let schema = indoc! {
            r#"model Test {
                #id(id, Int, @id)
                str String? @default("SOME_DEFAULT")
              }"#
        };

        schema.to_owned()
    }

    // "createMany" should "correctly use defaults and nulls"
    #[connector_test(schema(schema_3))]
    async fn create_many_defaults_nulls(runner: Runner) -> TestResult<()> {
        // Not providing a value must provide the default, providing null must result in null.
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createManyTest(data: [
              { id: 1 },
              { id: 2, str: null }
            ]) {
              count
            }
          }"#),
          @r###"{"data":{"createManyTest":{"count":2}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyTest {
              id
              str
            }
          }"#),
          @r###"{"data":{"findManyTest":[{"id":1,"str":"SOME_DEFAULT"},{"id":2,"str":null}]}}"###
        );

        Ok(())
    }

    fn schema_4() -> String {
        let schema = indoc! {
            r#"model Test {
                #id(id, Int, @id)
              }"#
        };

        schema.to_owned()
    }

    // "createMany" should "error on duplicates by default"
    #[connector_test(schema(schema_4))]
    async fn create_many_error_dups(runner: Runner) -> TestResult<()> {
        assert_error!(
            &runner,
            r#"mutation {
            createManyTest(data: [
              { id: 1 },
              { id: 1 }
            ]) {
              count
            }
          }"#,
            2002,
            "Unique constraint failed"
        );

        Ok(())
    }

    // "createMany" should "not error on duplicates with skipDuplicates true"
    #[connector_test(schema(schema_4), capabilities(CreateMany, CreateSkipDuplicates))]
    async fn create_many_no_error_skip_dup(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createManyTest(skipDuplicates: true, data: [
              { id: 1 },
              { id: 1 }
            ]) {
              count
            }
          }"#),
          @r###"{"data":{"createManyTest":{"count":1}}}"###
        );

        Ok(())
    }

    // "createMany" should "allow creating a large number of records (horizontal partitioning check)"
    // Note: Checks were originally higher, but test method (command line args) blows up...
    // Covers: Batching by row number.
    // Each DB allows a certain amount of params per single query, and a certain number of rows.
    // Each created row has 1 param and we create 1000 records.
    #[connector_test(schema(schema_4))]
    async fn large_num_records_horizontal(runner: Runner) -> TestResult<()> {
        let mut records: Vec<String> = vec![];

        for i in 1..=1000 {
            records.push(format!("{{ id: {i} }}"));
        }

        insta::assert_snapshot!(
          run_query!(&runner, format!(r#"mutation {{
            createManyTest(data: [{}]) {{
              count
            }}
          }}"#, records.join(", "))),
          @r###"{"data":{"createManyTest":{"count":1000}}}"###
        );

        Ok(())
    }

    fn schema_5() -> String {
        let schema = indoc! {
            r#"model Test {
                #id(id, Int, @id)
                a  Int
                b  Int
                c  Int
              }"#
        };

        schema.to_owned()
    }

    // "createMany" should "allow creating a large number of records (vertical partitioning check)"
    // Note: Checks were originally higher, but test method (command line args) blows up...
    // Covers: Batching by row number.
    // Each DB allows a certain amount of params per single query, and a certain number of rows.
    // Each created row has 4 params and we create 1000 rows.
    #[connector_test(schema(schema_5))]
    async fn large_num_records_vertical(runner: Runner) -> TestResult<()> {
        let mut records: Vec<String> = vec![];

        for i in 1..=2000 {
            records.push(format!("{{ id: {i}, a: {i}, b: {i}, c: {i} }}"));
        }

        insta::assert_snapshot!(
          run_query!(&runner, format!(r#"mutation {{
              createManyTest(data: [{}]) {{
                count
              }}
            }}"#, records.join(", "))),
          @r###"{"data":{"createManyTest":{"count":2000}}}"###
        );

        Ok(())
    }

    fn schema_6() -> String {
        let schema = indoc! {
            r#"
          model TestModel {
              #id(id, Int, @id)
              updatedAt DateTime @map("updated_at")
          }
          "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_6))]
    async fn create_many_map_behavior(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, format!(r#"mutation {{
              createManyTestModel(data: [
                {{ id: 1, updatedAt: "{}" }},
                {{ id: 2, updatedAt: "{}" }}
              ]) {{
                count
              }}
            }}"#, date_iso_string(2009, 8, 1), now())),
          @r###"{"data":{"createManyTestModel":{"count":2}}}"###
        );

        Ok(())
    }
}

#[test_suite(schema(json_opt), exclude(MySql(5.6)), capabilities(CreateMany, Json))]
mod json_create_many {
    use query_engine_tests::{assert_error, run_query};

    #[connector_test(only(MongoDb))]
    async fn create_many_json(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
              createManyTestModel(data: [
                { id: 1, json: "{}" },
                { id: 2, json: "null" },
                { id: 3, json: null },
                { id: 4 },
              ]) {
                count
              }
            }"#),
          @r###"{"data":{"createManyTestModel":{"count":4}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
              findManyTestModel {
                id
                json
              }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1,"json":"{}"},{"id":2,"json":null},{"id":3,"json":null},{"id":4,"json":null}]}}"###
        );

        Ok(())
    }

    #[connector_test(capabilities(AdvancedJsonNullability))]
    async fn create_many_json_adv(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
              createManyTestModel(data: [
                { id: 1, json: "{}" },
                { id: 2, json: JsonNull },
                { id: 3, json: DbNull },
                { id: 4 },
              ]) {
                count
              }
            }"#),
          @r###"{"data":{"createManyTestModel":{"count":4}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
              findManyTestModel {
                id
                json
              }
            }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1,"json":"{}"},{"id":2,"json":"null"},{"id":3,"json":null},{"id":4,"json":null}]}}"###
        );

        Ok(())
    }

    #[connector_test(capabilities(AdvancedJsonNullability))]
    async fn create_many_json_errors(runner: Runner) -> TestResult<()> {
        assert_error!(
            &runner,
            r#"mutation {
                  createManyTestModel(data: [
                    { id: 1, json: null },
                  ]) {
                    count
                  }
                }"#,
            2009,
            "A value is required but not set."
        );

        assert_error!(
            &runner,
            r#"mutation {
                createManyTestModel(data: [
                  { id: 1, json: AnyNull },
                ]) {
                  count
                }
              }"#,
            2009,
            "Enum value 'AnyNull' is invalid for enum type NullableJsonNullValueInput"
        );

        Ok(())
    }
}
