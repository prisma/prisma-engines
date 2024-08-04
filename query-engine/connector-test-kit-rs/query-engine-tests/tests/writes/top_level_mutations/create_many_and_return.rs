use query_engine_tests::*;

#[test_suite(capabilities(CreateMany, InsertReturning))]
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
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_1))]
    async fn basic_create_many(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createManyTestAndReturn(data: [
              { id: 1, str1: "1", str2: "1", str3: "1"},
              { id: 2, str1: "2",            str3: null},
              { id: 3, str1: "1"},
            ]) {
              id str1 str2 str3
            }
          }"#),
          @r###"{"data":{"createManyTestAndReturn":[{"id":1,"str1":"1","str2":"1","str3":"1"},{"id":2,"str1":"2","str2":null,"str3":null},{"id":3,"str1":"1","str2":null,"str3":"SOME_DEFAULT"}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(schema_1))]
    async fn basic_create_many_shorthand(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createManyTestAndReturn(data: { id: 1, str1: "1", str2: "1", str3: "1"}) {
              str1 str2 str3
            }
          }"#),
          @r###"{"data":{"createManyTestAndReturn":[{"str1":"1","str2":"1","str3":"1"}]}}"###
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
        capabilities(CreateManyWriteableAutoIncId, InsertReturning),
        exclude(CockroachDb)
    )]
    async fn basic_create_many_autoincrement(runner: Runner) -> TestResult<()> {
        let res = run_query_json!(
            &runner,
            r#"mutation {
              createManyTestAndReturn(data: [
                { id: 123, str1: "1", str2: "1", str3: "1"},
                { id: 321, str1: "2",            str3: null},
                {          str1: "1"},
              ]) {
                id str1 str2 str3
              }
            }"#,
            &["data", "createManyTestAndReturn"]
        );

        let mut res = match res {
            serde_json::Value::Array(items) => items,
            _ => panic!("Expected an array"),
        };

        // Order is not deterministic on SQLite
        res.sort_by_key(|x| x["id"].as_i64().unwrap());

        let json_res_string = serde_json::Value::Array(res).to_string();

        is_one_of!(
            json_res_string,
            [
                r#"[{"id":1,"str1":"1","str2":null,"str3":"SOME_DEFAULT"},{"id":123,"str1":"1","str2":"1","str3":"1"},{"id":321,"str1":"2","str2":null,"str3":null}]"#,
                // Sqlite sets the next autoincrement as MAX(id) + 1
                r#"[{"id":123,"str1":"1","str2":"1","str3":"1"},{"id":321,"str1":"2","str2":null,"str3":null},{"id":322,"str1":"1","str2":null,"str3":"SOME_DEFAULT"}]"#
            ]
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
            createManyTestAndReturn(data: [
              { id: 123, str1: "1", str2: "1", str3: "1"},
              { id: 321, str1: "2",            str3: null},
              {          str1: "1"},
            ]) {
              str1 str2 str3
            }
          }"#),
          @r###"{"data":{"createManyTestAndReturn":[{"str1":"1","str2":"1","str3":"1"},{"str1":"2","str2":null,"str3":null},{"str1":"1","str2":null,"str3":"SOME_DEFAULT"}]}}"###
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
            createManyTestAndReturn(data: [
              { id: 1 },
              { id: 2, str: null }
            ]) {
              id str
            }
          }"#),
          @r###"{"data":{"createManyTestAndReturn":[{"id":1,"str":"SOME_DEFAULT"},{"id":2,"str":null}]}}"###
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
            createManyTestAndReturn(data: [
              { id: 1 },
              { id: 1 }
            ]) {
              id
            }
          }"#,
            2002,
            "Unique constraint failed"
        );

        Ok(())
    }

    // "createMany" should "not error on duplicates with skipDuplicates true"
    #[connector_test(schema(schema_4), capabilities(CreateMany, CreateSkipDuplicates, InsertReturning))]
    async fn create_many_no_error_skip_dup(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createManyTestAndReturn(skipDuplicates: true, data: [
              { id: 1 },
              { id: 1 }
            ]) {
              id
            }
          }"#),
          @r###"{"data":{"createManyTestAndReturn":[{"id":1}]}}"###
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

        let res = run_query_json!(
            &runner,
            format!(
                r#"mutation {{
                  createManyTestAndReturn(data: [{}]) {{
                    id
                  }}
                }}"#,
                records.join(", ")
            ),
            &["data", "createManyTestAndReturn"]
        );

        assert_eq!(res.as_array().map(|a| a.len()), Some(1000));

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

        let res = run_query_json!(
            &runner,
            format!(
                r#"mutation {{
                  createManyTestAndReturn(data: [{}]) {{
                    a b c
                  }}
              }}"#,
                records.join(", ")
            ),
            &["data", "createManyTestAndReturn"]
        );

        assert_eq!(res.as_array().map(|a| a.len()), Some(2000));

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
              createManyTestModelAndReturn(data: [
                {{ id: 1, updatedAt: "{}" }},
                {{ id: 2, updatedAt: "{}" }}
              ]) {{
                id updatedAt
              }}
            }}"#, date_iso_string(2009, 8, 1), date_iso_string(1337, 1, 1))),
          @r###"{"data":{"createManyTestModelAndReturn":[{"id":1,"updatedAt":"2009-08-01T00:00:00.000Z"},{"id":2,"updatedAt":"1337-01-01T00:00:00.000Z"}]}}"###
        );

        Ok(())
    }

    fn schema_11_child() -> String {
        let schema = indoc! {
            r#"model Test {
              #id(id, Int, @id)

              child Child?
            }
            
            model Child {
              #id(id, Int, @id)

              testId Int? @unique
              test Test? @relation(fields: [testId], references: [id])
            
            }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_1m_child))]
    async fn create_many_11_inline_rel_read_works(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createManyTestAndReturn(data: [{ id: 1 }, { id: 2 }]) { id } }"#),
          @r###"{"data":{"createManyTestAndReturn":[{"id":1},{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createManyChildAndReturn(data: [
              { id: 1,  testId: 1 },
              { id: 2,  testId: 2 },
              { id: 3,            },
            ]) { id test { id } }
          }"#),
          @r###"{"data":{"createManyChildAndReturn":[{"id":1,"test":{"id":1}},{"id":2,"test":{"id":2}},{"id":3,"test":null}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(schema_11_child))]
    async fn create_many_11_non_inline_rel_read_fails(runner: Runner) -> TestResult<()> {
        runner
            .query_json(serde_json::json!({
              "modelName": "Test",
              "action": "createManyAndReturn",
              "query": {
                "arguments": { "data": { "id": 1 } },
                "selection": {
                  "id": true,
                  "child": true
                }
              }
            }))
            .await?
            .assert_failure(2009, Some("Field 'child' not found in enclosing type".to_string()));

        Ok(())
    }

    fn schema_1m_child() -> String {
        let schema = indoc! {
            r#"model Test {
              #id(id, Int, @id)
              str1 String?
              str2 String?
              str3 String? @default("SOME_DEFAULT")

              children Child[]
            }
            
            model Child {
              #id(id, Int, @id)
              str1 String?
              str2 String?
              str3 String? @default("SOME_DEFAULT")

              testId Int?
              test Test? @relation(fields: [testId], references: [id])
            
            }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_1m_child))]
    async fn create_many_1m_inline_rel_read_works(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createManyTestAndReturn(data: [{ id: 1, str1: "1" }, { id: 2, str1: "2" }]) { id str1 str2 str3 } }"#),
          @r###"{"data":{"createManyTestAndReturn":[{"id":1,"str1":"1","str2":null,"str3":"SOME_DEFAULT"},{"id":2,"str1":"2","str2":null,"str3":"SOME_DEFAULT"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createManyChildAndReturn(data: [
              { id: 1, str1: "1", str2: "1", str3: "1",  testId: 1 },
              { id: 2, str1: "2",            str3: null, testId: 2 },
              { id: 3, str1: "1"                                   },
            ]) { id str1 str2 str3 test { id str1 str2 str3 } }
          }"#),
          @r###"{"data":{"createManyChildAndReturn":[{"id":1,"str1":"1","str2":"1","str3":"1","test":{"id":1,"str1":"1","str2":null,"str3":"SOME_DEFAULT"}},{"id":2,"str1":"2","str2":null,"str3":null,"test":{"id":2,"str1":"2","str2":null,"str3":"SOME_DEFAULT"}},{"id":3,"str1":"1","str2":null,"str3":"SOME_DEFAULT","test":null}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(schema_1m_child))]
    async fn create_many_1m_non_inline_rel_read_fails(runner: Runner) -> TestResult<()> {
        runner
            .query_json(serde_json::json!({
              "modelName": "Test",
              "action": "createManyAndReturn",
              "query": {
                "arguments": { "data": { "id": 1 } },
                "selection": {
                  "id": true,
                  "children": true
                }
              }
            }))
            .await?
            .assert_failure(2009, Some("Field 'children' not found in enclosing type".to_string()));

        Ok(())
    }

    fn schema_m2m_child() -> String {
        let schema = indoc! {
            r#"model Test {
            #id(id, Int, @id)
            str1 String?

            #m2m(children, Child[], id, Int)
          }
          
          model Child {
            #id(id, Int, @id)

            #m2m(tests, Test[], id, Int)
          
          }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_m2m_child))]
    async fn create_many_m2m_rel_read_fails(runner: Runner) -> TestResult<()> {
        runner
            .query_json(serde_json::json!({
              "modelName": "Test",
              "action": "createManyAndReturn",
              "query": {
                "arguments": { "data": { "id": 1 } },
                "selection": {
                  "id": true,
                  "children": true
                }
              }
            }))
            .await?
            .assert_failure(2009, Some("Field 'children' not found in enclosing type".to_string()));

        runner
            .query_json(serde_json::json!({
              "modelName": "Child",
              "action": "createManyAndReturn",
              "query": {
                "arguments": { "data": { "id": 1 } },
                "selection": {
                  "id": true,
                  "tests": true
                }
              }
            }))
            .await?
            .assert_failure(2009, Some("Field 'tests' not found in enclosing type".to_string()));

        Ok(())
    }

    fn schema_self_rel_child() -> String {
        let schema = indoc! {
            r#"model Child {
              #id(id, Int, @id)
            
              teacherId Int?
              teacher   Child?  @relation("TeacherStudents", fields: [teacherId], references: [id])
              students  Child[] @relation("TeacherStudents")
            }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_self_rel_child))]
    async fn create_many_self_rel_read_fails(runner: Runner) -> TestResult<()> {
        runner
            .query_json(serde_json::json!({
              "modelName": "Child",
              "action": "createManyAndReturn",
              "query": {
                "arguments": { "data": { "id": 1 } },
                "selection": {
                  "id": true,
                  "students": true
                }
              }
            }))
            .await?
            .assert_failure(2009, Some("Field 'students' not found in enclosing type".to_string()));

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

    #[connector_test(schema(schema_7))]
    async fn create_many_by_shape(runner: Runner) -> TestResult<()> {
        // Generated queries for SQLite:
        // INSERT INTO `main`.`Test` (`opt`, `req`) VALUES (null, ?), (?, ?) params=[1,2,2]
        // INSERT INTO `main`.`Test` (`opt_default`, `opt`, `req`) VALUES (?, null, ?), (?, ?, ?) params=[3,3,6,6,6]
        // INSERT INTO `main`.`Test` (`req_default`, `opt_default`, `req`, `opt`) VALUES (?, ?, ?, null), (?, ?, ?, ?) params=[5,5,5,7,7,7,7]
        // INSERT INTO `main`.`Test` (`req`, `req_default`, `opt`) VALUES (?, ?, ?) params=[4,4,4]

        let res = run_query_json!(
            &runner,
            r#"mutation {
            createManyTestAndReturn(
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
              req req_default req_default_static
              opt opt_default opt_default_static
            }
          }"#,
            &["data", "createManyTestAndReturn"]
        );

        let mut items = match res {
            serde_json::Value::Array(items) => items,
            _ => panic!("Expected an array"),
        };

        // Order is not deterministic on SQLite
        items.sort_by_key(|x| x["req"].as_i64().unwrap());

        insta::assert_snapshot!(
          serde_json::Value::Array(items).to_string(),
          @r###"[{"req":1,"req_default":1,"req_default_static":1,"opt":null,"opt_default":1,"opt_default_static":1},{"req":2,"req_default":1,"req_default_static":1,"opt":2,"opt_default":1,"opt_default_static":1},{"req":3,"req_default":1,"req_default_static":1,"opt":null,"opt_default":3,"opt_default_static":1},{"req":4,"req_default":4,"req_default_static":1,"opt":4,"opt_default":1,"opt_default_static":1},{"req":5,"req_default":5,"req_default_static":1,"opt":null,"opt_default":5,"opt_default_static":1},{"req":6,"req_default":1,"req_default_static":1,"opt":6,"opt_default":6,"opt_default_static":1},{"req":7,"req_default":7,"req_default_static":1,"opt":7,"opt_default":7,"opt_default_static":1}]"###
        );

        Ok(())
    }

    #[connector_test(schema(schema_7), only(Sqlite))]
    async fn create_many_by_shape_combinations(runner: Runner) -> TestResult<()> {
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
                format!(r#"mutation {{ createManyTestAndReturn(data: [{data}]) {{ req }} }}"#)
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
              createManyTestAndReturn(
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
                req
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
              createManyTestAndReturn(
                data: [
                  { req: 1 }
                  { req: 2, opt_default_static: 2 },
                  { req: 3, req_default_static: 3 },
                  { req: 4, opt: 8 },
                  { req: 5, opt: null },
                ]
              ) {
                req
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
              createManyTestAndReturn(
                data: [
                  { req: 1 }
                  { req: 2, opt_default_static: 2 },
                  { req: 3, req_default_static: 3 },
                  { req: 4, opt: 8 },
                  { req: 5, opt: null },
                  { req: 6, opt_default: 3 },
                ]
              ) {
                req
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

#[test_suite(
    schema(json_opt),
    exclude(MySql(5.6)),
    capabilities(Json, AdvancedJsonNullability, CreateMany, InsertReturning)
)]
mod json_create_many {
    use query_engine_tests::{assert_error, run_query};

    #[connector_test]
    async fn create_many_json_adv(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
              createManyTestModelAndReturn(data: [
                { id: 1, json: "{}" },
                { id: 2, json: JsonNull },
                { id: 3, json: DbNull },
                { id: 4 },
              ]) {
                id json
              }
            }"#),
          @r###"{"data":{"createManyTestModelAndReturn":[{"id":1,"json":"{}"},{"id":2,"json":"null"},{"id":3,"json":null},{"id":4,"json":null}]}}"###
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

    #[connector_test]
    async fn create_many_json_errors(runner: Runner) -> TestResult<()> {
        assert_error!(
            &runner,
            r#"mutation {
                createManyTestModelAndReturn(data: [
                  { id: 1, json: AnyNull },
                ]) {
                  id json
                }
              }"#,
            2009,
            "`AnyNull` is not a valid `NullableJsonNullValueInput`"
        );

        Ok(())
    }
}
