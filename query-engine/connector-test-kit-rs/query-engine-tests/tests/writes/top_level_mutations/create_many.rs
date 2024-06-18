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
    #[connector_test(
        schema(schema_2),
        capabilities(CreateManyWriteableAutoIncId),
        exclude(CockroachDb, Sqlite("cfd1"))
    )]
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
    // TODO: unexclude d1 once https://github.com/prisma/team-orm/issues/1070 is fixed
    #[connector_test(schema(schema_4), exclude(Sqlite("cfd1")))]
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
    // TODO: unexclude d1 once https://github.com/prisma/team-orm/issues/1070 is fixed
    #[connector_test(schema(schema_5), exclude(Sqlite("cfd1")))]
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

    fn schema_7() -> String {
        let schema = indoc! {
          r#"model Test {
            req Int @id
            req_default Int @default(dbgenerated("1"))
            req_default_static Int @default(1)
            opt Int?
            opt_default Int? @default(dbgenerated("1"))
            opt_default_static Int? @default(1)
          }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_7), only(Sqlite))]
    async fn create_many_by_shape(runner: Runner) -> TestResult<()> {
        use itertools::Itertools;

        let mut id = 1;

        // Generates a powerset of all combinations of these fields
        // In an attempt to ensure that we never generate invalid insert statements
        // because of the grouping logic.
        for sets in vec!["req_default", "opt", "opt_default"]
            .into_iter()
            .powerset()
            .map(|mut set| {
                set.extend_from_slice(&["req"]);
                set
            })
            .powerset()
        {
            let data = sets
                .into_iter()
                .map(|set| {
                    let res = set.into_iter().map(|field| format!("{field}: {id}")).join(", ");

                    id += 1;

                    format!("{{ {res} }}")
                })
                .join(", ");

            run_query!(
                &runner,
                format!(r#"mutation {{ createManyTest(data: [{data}]) {{ count }} }}"#)
            );
        }

        Ok(())
    }

    // LibSQL & co are ignored because they don't support metrics
    #[connector_test(schema(schema_7), only(Sqlite("3")))]
    async fn create_many_by_shape_counter_1(runner: Runner) -> TestResult<()> {
        use query_engine_metrics::PRISMA_DATASOURCE_QUERIES_TOTAL;

        // Generated queries:
        // INSERT INTO `main`.`Test` (`opt`, `req`) VALUES (null, ?), (?, ?) params=[1,2,2]
        // INSERT INTO `main`.`Test` (`opt_default`, `opt`, `req`) VALUES (?, null, ?), (?, ?, ?) params=[3,3,6,6,6]
        // INSERT INTO `main`.`Test` (`req_default`, `opt_default`, `req`, `opt`) VALUES (?, ?, ?, null), (?, ?, ?, ?) params=[5,5,5,7,7,7,7]
        // INSERT INTO `main`.`Test` (`req`, `req_default`, `opt`) VALUES (?, ?, ?) params=[4,4,4]
        run_query!(
            &runner,
            r#"mutation {
              createManyTest(
                data: [
                  { req: 1 }
                  { opt: 2, req: 2 }
                  { opt_default: 3, req: 3 }
                  { req_default: 4, opt: 4, req: 4 }
                  { req_default: 5, opt_default: 5, req: 5 }
                  { opt: 6, opt_default: 6, req: 6 }
                  { req_default: 7, opt: 7, opt_default: 7, req: 7 }
                ]
              ) {
                count
              }
            }"#
        );

        let json = runner.get_metrics().to_json(Default::default());
        let counter = metrics::get_counter(&json, PRISMA_DATASOURCE_QUERIES_TOTAL);

        match runner.max_bind_values() {
            Some(x) if x > 18 => assert_eq!(counter, 6), // 4 queries in total (BEGIN/COMMIT are counted)
            // Some queries are being split because of `QUERY_BATCH_SIZE` being set to `10` in dev.
            Some(_) => assert_eq!(counter, 7), // 5 queries in total (BEGIN/COMMIT are counted)
            _ => panic!("Expected max bind values to be set"),
        }

        Ok(())
    }

    // LibSQL & co are ignored because they don't support metrics
    #[connector_test(schema(schema_7), only(Sqlite("3")))]
    async fn create_many_by_shape_counter_2(runner: Runner) -> TestResult<()> {
        use query_engine_metrics::PRISMA_DATASOURCE_QUERIES_TOTAL;

        // Generated queries:
        // INSERT INTO `main`.`Test` ( `opt_default_static`, `req_default_static`, `opt`, `req` ) VALUES (?, ?, null, ?), (?, ?, null, ?), (?, ?, null, ?) params=[1,1,1,2,1,2,1,3,3]
        // INSERT INTO `main`.`Test` ( `opt_default_static`, `req_default_static`, `opt`, `req` ) VALUES (?, ?, ?, ?), (?, ?, ?, ?) params=[1,1,8,4,1,1,null,5]
        // Note: Two queries are generated because QUERY_BATCH_SIZE is set to 10. In production, a single query would be generated for this example.
        run_query!(
            &runner,
            r#"mutation {
              createManyTest(
                data: [
                  { req: 1 }
                  { req: 2, opt_default_static: 2 },
                  { req: 3, req_default_static: 3 },
                  { req: 4, opt: 8 },
                  { req: 5, opt: null },
                ]
              ) {
                count
              }
            }"#
        );

        let json = runner.get_metrics().to_json(Default::default());
        let counter = metrics::get_counter(&json, PRISMA_DATASOURCE_QUERIES_TOTAL);

        match runner.max_bind_values() {
            Some(x) if x >= 18 => assert_eq!(counter, 3), // 1 createMany queries (BEGIN/COMMIT are counted)
            // Some queries are being split because of `QUERY_BATCH_SIZE` being set to `10` in dev.
            Some(_) => assert_eq!(counter, 4), // 2 createMany queries (BEGIN/COMMIT are counted)
            _ => panic!("Expected max bind values to be set"),
        }

        Ok(())
    }

    // LibSQL & co are ignored because they don't support metrics
    #[connector_test(schema(schema_7), only(Sqlite("3")))]
    async fn create_many_by_shape_counter_3(runner: Runner) -> TestResult<()> {
        use query_engine_metrics::PRISMA_DATASOURCE_QUERIES_TOTAL;

        // Generated queries:
        // INSERT INTO `main`.`Test` ( `req_default_static`, `req`, `opt_default`, `opt_default_static` ) VALUES (?, ?, ?, ?) params=[1,6,3,1]
        // INSERT INTO `main`.`Test` ( `opt`, `req`, `req_default_static`, `opt_default_static` ) VALUES (null, ?, ?, ?), (null, ?, ?, ?), (null, ?, ?, ?) params=[1,1,1,2,1,2,3,3,1]
        // INSERT INTO `main`.`Test` ( `opt`, `req`, `req_default_static`, `opt_default_static` ) VALUES (?, ?, ?, ?), (?, ?, ?, ?) params=[8,4,1,1,null,5,1,1]
        // Note: The first two queries are split because QUERY_BATCH_SIZE is set to 10. In production, only two queries would be generated for this example.
        run_query!(
            &runner,
            r#"mutation {
              createManyTest(
                data: [
                  { req: 1 }
                  { req: 2, opt_default_static: 2 },
                  { req: 3, req_default_static: 3 },
                  { req: 4, opt: 8 },
                  { req: 5, opt: null },
                  { req: 6, opt_default: 3 },
                ]
              ) {
                count
              }
            }"#
        );

        let json = runner.get_metrics().to_json(Default::default());
        let counter = metrics::get_counter(&json, PRISMA_DATASOURCE_QUERIES_TOTAL);

        match runner.max_bind_values() {
            Some(x) if x > 21 => assert_eq!(counter, 4), // 3 createMany queries in total (BEGIN/COMMIT are counted)
            // Some queries are being split because of `QUERY_BATCH_SIZE` being set to `10` in dev.
            Some(_) => assert_eq!(counter, 5), // 3 createMany queries in total (BEGIN/COMMIT are counted)
            _ => panic!("Expected max bind values to be set"),
        }

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
                  { id: 1, json: AnyNull },
                ]) {
                  count
                }
              }"#,
            2009,
            "`AnyNull` is not a valid `NullableJsonNullValueInput`"
        );

        Ok(())
    }
}
