use query_engine_tests::*;

#[test_suite]
mod update {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query, run_query_json, TROUBLE_CHARS};

    fn schema_1() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              optString   String?
              optInt      Int?
              optFloat    Float?
              optBoolean  Boolean?
              optDateTime DateTime?
            }"#
        };

        schema.to_owned()
    }

    fn schema_2() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              strField  String
              uniqField String? @unique
            }"#
        };

        schema.to_owned()
    }

    fn schema_3() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              optEnum MyEnum?
            }

            enum MyEnum {
              A
              ABCD
            }"#
        };

        schema.to_owned()
    }

    fn schema_4() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              field     String
              updatedAt DateTime @updatedAt
              createdAt DateTime @default(now())
            }"#
        };

        schema.to_owned()
    }

    fn schema_5() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              createdAt DateTime @default(now())
              updatedAt DateTime @updatedAt
            }"#
        };

        schema.to_owned()
    }

    fn schema_6() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              optInt   Int?
              optFloat Float?
            }"#
        };

        schema.to_owned()
    }

    fn schema_7() -> String {
        let schema = indoc! {
            r#"model TestModel {
              id1  Float
              id2  Int
              uniq Int @unique

              @@id([id1, id2])
            }"#
        };

        schema.to_owned()
    }

    fn schema_8() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              test                  Int?
              updatedAt_w_default   DateTime  @default(now()) @updatedAt
              updatedAt_wo_default  DateTime? @updatedAt
              createdAt             DateTime  @default(now())
            }"#
        };

        schema.to_owned()
    }

    // More than one "updateAt" is being updated by the QE
    // a default value does not influence it being updated
    #[connector_test(schema(schema_8))]
    async fn updated_at_with_default(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1}"#).await?;
        create_row(&runner, r#"{ id: 2}"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTestModel(
              where: { id: 1 }
              data: {
                test: 1
              }
            ) {
              id
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query {
            findManyTestModel(
              where: { AND: [{updatedAt_w_default: { gt: { _ref: "createdAt" } }},
                             {updatedAt_wo_default: { gt: { _ref: "createdAt" } }},
                             {updatedAt_wo_default: { equals: { _ref: "updatedAt_w_default" } }}
                     ]}
            ) {
              id
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        Ok(())
    }

    //"An updateOne mutation" should "update an item"
    #[connector_test(schema(schema_1))]
    async fn update_an_item(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1 }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, format!(r#"mutation {{
            updateOneTestModel(
              where: {{ id: 1 }}
              data: {{
                optString: {{ set: "test{TROUBLE_CHARS}" }}
                optInt: {{ set: 1337 }}
                optFloat: {{ set: 1.234 }}
                optBoolean: {{ set: true }}
                optDateTime: {{ set: "2016-07-31T23:59:01.000Z" }}
              }}
            ) {{
              optString
              optInt
              optFloat
              optBoolean
              optDateTime
            }}
          }}"#)),
          @r###"{"data":{"updateOneTestModel":{"optString":"test¥฿😀😁😂😃😄😅😆😇😈😉😊😋😌😍😎😏😐😑😒😓😔😕😖😗😘😙😚😛😜😝😞😟😠😡😢😣😤😥😦😧😨😩😪😫😬😭😮😯😰😱😲😳😴😵😶😷😸😹😺😻😼😽😾😿🙀🙁🙂🙃🙄🙅🙆🙇🙈🙉🙊🙋🙌🙍🙎🙏ऀँंःऄअआइईउऊऋऌऍऎएऐऑऒओऔकखगघङचछजझञटठडढणतथदधनऩपफबभमयर€₭₮₯₰₱₲₳₴₵₶₷₸₹₺₻₼₽₾₿⃀","optInt":1337,"optFloat":1.234,"optBoolean":true,"optDateTime":"2016-07-31T23:59:01.000Z"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        Ok(())
    }

    // "An updateOne mutation" should "update an item with shorthand notation"
    #[connector_test(schema(schema_1))]
    async fn update_with_shorthand_notation(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1 }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, format!(r#"mutation {{
            updateOneTestModel(
              where: {{ id: 1 }}
              data: {{
                optString: "test{TROUBLE_CHARS}",
                optInt: 1337,
                optFloat: 1.234,
                optBoolean: true,
                optDateTime: "2016-07-31T23:59:01.000Z",
              }}
            ) {{
              optString
              optInt
              optFloat
              optBoolean
              optDateTime
            }}
          }}"#)),
          @r###"{"data":{"updateOneTestModel":{"optString":"test¥฿😀😁😂😃😄😅😆😇😈😉😊😋😌😍😎😏😐😑😒😓😔😕😖😗😘😙😚😛😜😝😞😟😠😡😢😣😤😥😦😧😨😩😪😫😬😭😮😯😰😱😲😳😴😵😶😷😸😹😺😻😼😽😾😿🙀🙁🙂🙃🙄🙅🙆🙇🙈🙉🙊🙋🙌🙍🙎🙏ऀँंःऄअआइईउऊऋऌऍऎएऐऑऒओऔकखगघङचछजझञटठडढणतथदधनऩपफबभमयर€₭₮₯₰₱₲₳₴₵₶₷₸₹₺₻₼₽₾₿⃀","optInt":1337,"optFloat":1.234,"optBoolean":true,"optDateTime":"2016-07-31T23:59:01.000Z"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        Ok(())
    }

    // "An updateOne mutation" should "update an item by a unique field"
    #[connector_test(schema(schema_2))]
    async fn update_by_uniq_field(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, strField: "test", uniqField: "uniq"}"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTestModel(
              where: { uniqField: "uniq" }
              data: { strField: { set: "updated" } }
            ){
              strField
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"strField":"updated"}}}"###
        );

        Ok(())
    }

    // "An updateOne mutation" should "update enums"
    // TODO: Flaky test on Cockroach, re-enable once figured out
    #[connector_test(schema(schema_3), capabilities(Enums), exclude(CockroachDb))]
    async fn update_enums(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1 }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTestModel(
              where: { id: 1 }
              data: { optEnum: { set: A } }
            ) {
              optEnum
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"optEnum":"A"}}}"###
        );

        Ok(())
    }

    // "An updateOne mutation" should "gracefully fail when trying to update an item by a unique field with a non-existing value"
    #[connector_test(schema(schema_2))]
    async fn update_fail_uniq_field_inexistant_value(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, strField: "test", uniqField: "uniq"}"#).await?;

        assert_error!(
          runner,
          r#"mutation {
            updateOneTestModel(
              where: { uniqField: "doesn't exist" }
              data: { strField: { set: "updated" } }
            ){
              id
            }
          }"#,
          2025,
          "An operation failed because it depends on one or more records that were required but not found. Record to update not found."
        );

        Ok(())
    }

    // "An updateOne mutation" should "update an updatedAt datetime"
    #[connector_test(schema(schema_4))]
    async fn update_updated_at_datetime(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, field: "test"}"#).await?;

        let res = run_query_json!(
            &runner,
            r#"mutation {
                updateOneTestModel(
                  where: { id: 1 }
                  data: { field: { set: "test2" } }
                ){
                  createdAt
                  updatedAt
                }
            }"#
        );
        let created_at = &res["data"]["updateOneTestModel"]["createdAt"].to_string();
        let updated_at = &res["data"]["updateOneTestModel"]["updatedAt"].to_string();

        assert_ne!(created_at, updated_at);

        Ok(())
    }

    // "UpdatedAt and createdAt" should "be mutable with an update"
    #[connector_test(schema(schema_5))]
    async fn updated_created_at_mutable_with_update(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTestModel(
              where: { id: 1 }
              data: {
                createdAt: { set: "2000-01-01T00:00:00Z" }
                updatedAt: { set: "2001-01-01T00:00:00Z" }
              }
            ) {
              createdAt
              updatedAt
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"createdAt":"2000-01-01T00:00:00.000Z","updatedAt":"2001-01-01T00:00:00.000Z"}}}"###
        );

        Ok(())
    }

    // "An updateOne mutation" should "correctly apply all number operations for Int"
    // TODO(dom): Not working on Mongo (first snapshot)
    // -{"data":{"updateOneTestModel":{"optInt":null}}}
    // +{"data":{"updateOneTestModel":{"optInt":10}}}
    #[connector_test(schema(schema_6), exclude(CockroachDb))]
    async fn update_apply_number_ops_for_int(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1 }"#).await?;
        create_row(&runner, r#"{ id: 2, optInt: 3}"#).await?;

        // Increment
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optInt", "increment", "10").await?,
          @r###"{"data":{"updateOneTestModel":{"optInt":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optInt", "increment", "10").await?,
          @r###"{"data":{"updateOneTestModel":{"optInt":13}}}"###
        );

        // Decrement
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optInt", "decrement", "10").await?,
          @r###"{"data":{"updateOneTestModel":{"optInt":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optInt", "decrement", "10").await?,
          @r###"{"data":{"updateOneTestModel":{"optInt":3}}}"###
        );

        // Multiply
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optInt", "multiply", "2").await?,
          @r###"{"data":{"updateOneTestModel":{"optInt":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optInt", "multiply", "2").await?,
          @r###"{"data":{"updateOneTestModel":{"optInt":6}}}"###
        );

        // Divide
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optInt", "divide", "3").await?,
          @r###"{"data":{"updateOneTestModel":{"optInt":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optInt", "divide", "3").await?,
          @r###"{"data":{"updateOneTestModel":{"optInt":2}}}"###
        );

        // Set
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optInt", "set", "5").await?,
          @r###"{"data":{"updateOneTestModel":{"optInt":5}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optInt", "set", "5").await?,
          @r###"{"data":{"updateOneTestModel":{"optInt":5}}}"###
        );

        // Set null
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optInt", "set", "null").await?,
          @r###"{"data":{"updateOneTestModel":{"optInt":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optInt", "set", "null").await?,
          @r###"{"data":{"updateOneTestModel":{"optInt":null}}}"###
        );

        Ok(())
    }

    // CockroachDB does not support the "divide" operator as is.
    // See https://github.com/cockroachdb/cockroach/issues/41448.
    #[connector_test(schema(schema_6), only(CockroachDb))]
    async fn update_apply_number_ops_for_int_cockroach(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1 }"#).await?;
        create_row(&runner, r#"{ id: 2, optInt: 3}"#).await?;

        // Increment
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optInt", "increment", "10").await?,
          @r###"{"data":{"updateOneTestModel":{"optInt":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optInt", "increment", "10").await?,
          @r###"{"data":{"updateOneTestModel":{"optInt":13}}}"###
        );

        // Decrement
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optInt", "decrement", "10").await?,
          @r###"{"data":{"updateOneTestModel":{"optInt":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optInt", "decrement", "10").await?,
          @r###"{"data":{"updateOneTestModel":{"optInt":3}}}"###
        );

        // Multiply
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optInt", "multiply", "2").await?,
          @r###"{"data":{"updateOneTestModel":{"optInt":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optInt", "multiply", "2").await?,
          @r###"{"data":{"updateOneTestModel":{"optInt":6}}}"###
        );

        // Set
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optInt", "set", "5").await?,
          @r###"{"data":{"updateOneTestModel":{"optInt":5}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optInt", "set", "5").await?,
          @r###"{"data":{"updateOneTestModel":{"optInt":5}}}"###
        );

        // Set null
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optInt", "set", "null").await?,
          @r###"{"data":{"updateOneTestModel":{"optInt":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optInt", "set", "null").await?,
          @r###"{"data":{"updateOneTestModel":{"optInt":null}}}"###
        );

        Ok(())
    }

    // "An updateOne mutation" should "correctly apply all number operations for Float"
    #[connector_test(schema(schema_6), exclude(MongoDb))]
    async fn update_apply_number_ops_for_float(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1 }"#).await?;
        create_row(&runner, r#"{ id: 2, optFloat: 5.5}"#).await?;

        // Increment
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optFloat", "increment", "4.6").await?,
          @r###"{"data":{"updateOneTestModel":{"optFloat":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optFloat", "increment", "4.6").await?,
          @r###"{"data":{"updateOneTestModel":{"optFloat":10.1}}}"###
        );

        // Decrement
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optFloat", "decrement", "4.6").await?,
          @r###"{"data":{"updateOneTestModel":{"optFloat":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optFloat", "decrement", "4.6").await?,
          @r###"{"data":{"updateOneTestModel":{"optFloat":5.5}}}"###
        );

        // Multiply
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optFloat", "multiply", "2").await?,
          @r###"{"data":{"updateOneTestModel":{"optFloat":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optFloat", "multiply", "2").await?,
          @r###"{"data":{"updateOneTestModel":{"optFloat":11.0}}}"###
        );

        // Divide
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optFloat", "divide", "2").await?,
          @r###"{"data":{"updateOneTestModel":{"optFloat":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optFloat", "divide", "2").await?,
          @r###"{"data":{"updateOneTestModel":{"optFloat":5.5}}}"###
        );

        // Set
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optFloat", "set", "5.1").await?,
          @r###"{"data":{"updateOneTestModel":{"optFloat":5.1}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optFloat", "set", "5.1").await?,
          @r###"{"data":{"updateOneTestModel":{"optFloat":5.1}}}"###
        );

        // Set null
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optFloat", "set", "null").await?,
          @r###"{"data":{"updateOneTestModel":{"optFloat":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optFloat", "set", "null").await?,
          @r###"{"data":{"updateOneTestModel":{"optFloat":null}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(schema_6), only(MongoDb))]
    async fn update_apply_number_ops_for_float_mongo(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1 }"#).await?;
        create_row(&runner, r#"{ id: 2, optFloat: 5.5}"#).await?;

        // Increment
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optFloat", "increment", "4.6").await?,
          @r###"{"data":{"updateOneTestModel":{"optFloat":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optFloat", "increment", "4.6").await?,
          @r###"{"data":{"updateOneTestModel":{"optFloat":10.1}}}"###
        );

        // Decrement
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optFloat", "decrement", "4.6").await?,
          @r###"{"data":{"updateOneTestModel":{"optFloat":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optFloat", "decrement", "4.6").await?,
          @r###"{"data":{"updateOneTestModel":{"optFloat":5.5}}}"###
        );

        // Multiply
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optFloat", "multiply", "2").await?,
          @r###"{"data":{"updateOneTestModel":{"optFloat":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optFloat", "multiply", "2").await?,
          @r###"{"data":{"updateOneTestModel":{"optFloat":11.0}}}"###
        );

        // Divide
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optFloat", "divide", "2").await?,
          @r###"{"data":{"updateOneTestModel":{"optFloat":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optFloat", "divide", "2").await?,
          @r###"{"data":{"updateOneTestModel":{"optFloat":5.5}}}"###
        );

        // Set
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optFloat", "set", "5.1").await?,
          @r###"{"data":{"updateOneTestModel":{"optFloat":5.1}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optFloat", "set", "5.1").await?,
          @r###"{"data":{"updateOneTestModel":{"optFloat":5.1}}}"###
        );

        // Set null
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optFloat", "set", "null").await?,
          @r###"{"data":{"updateOneTestModel":{"optFloat":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optFloat", "set", "null").await?,
          @r###"{"data":{"updateOneTestModel":{"optFloat":null}}}"###
        );

        Ok(())
    }

    // "An updateOne mutation with number operations" should "handle id changes correctly"
    #[connector_test(schema(schema_7), capabilities(CompoundIds))]
    async fn update_number_ops_handle_id_change(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation { createOneTestModel(data: { id1: 1.23456, id2: 2, uniq: 3 }) { id1 } }"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTestModel(
              where: { id1_id2: { id1: 1.23456, id2: 2 } }
              data: {
                id1: { divide: 2 }
                uniq: { multiply: 3 }
              }
            ){
              id1
              id2
              uniq
            }
          }"#),
          @r###"{"data":{"updateOneTestModel":{"id1":0.61728,"id2":2,"uniq":9}}}"###
        );

        Ok(())
    }

    async fn query_number_operation(
        runner: &Runner,
        id: &str,
        field: &str,
        op: &str,
        value: &str,
    ) -> TestResult<String> {
        let res = run_query!(
            runner,
            format!(
                r#"mutation {{
              updateOneTestModel(
                where: {{ id: {id} }}
                data: {{ {field}: {{ {op}: {value} }} }}
              ){{
                {field}
              }}
            }}"#
            )
        );

        Ok(res)
    }

    #[connector_test(schema(generic))]
    async fn update_fails_if_filter_dont_match(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation { createOneTestModel(data: { id: 1, field: "hello" }) { id } }"#
        );

        assert_error!(
            &runner,
            r#"mutation {
                  updateOneTestModel(where: { id: 1, field: "bonjour" }, data: { field: "updated" }) {
                    id
                  }
                }"#,
            2025,
            "An operation failed because it depends on one or more records that were required but not found. Record to update not found."
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTestModel(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}

#[test_suite(schema(json_opt), exclude(MySql(5.6)), capabilities(Json))]
mod json_update {
    use query_engine_tests::{assert_error, run_query};

    #[connector_test(only(MongoDb))]
    async fn update_json(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 1 }) { json }}"#),
          @r###"{"data":{"createOneTestModel":{"json":null}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(where: { id: 1 }, data: { json: "{}" }) { json }}"#),
          @r###"{"data":{"updateOneTestModel":{"json":"{}"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(where: { id: 1 }, data: { json: "null" }) { json }}"#),
          @r###"{"data":{"updateOneTestModel":{"json":null}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(where: { id: 1 }, data: { json: null }) { json }}"#),
          @r###"{"data":{"updateOneTestModel":{"json":null}}}"###
        );

        Ok(())
    }

    #[connector_test(capabilities(AdvancedJsonNullability))]
    async fn update_json_adv(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 1 }) { json }}"#),
          @r###"{"data":{"createOneTestModel":{"json":null}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(where: { id: 1 }, data: { json: "{}" }) { json }}"#),
          @r###"{"data":{"updateOneTestModel":{"json":"{}"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(where: { id: 1 }, data: { json: JsonNull }) { json }}"#),
          @r###"{"data":{"updateOneTestModel":{"json":"null"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(where: { id: 1 }, data: { json: DbNull }) { json }}"#),
          @r###"{"data":{"updateOneTestModel":{"json":null}}}"###
        );

        Ok(())
    }

    #[connector_test(capabilities(AdvancedJsonNullability))]
    async fn update_json_errors(runner: Runner) -> TestResult<()> {
        assert_error!(
            &runner,
            r#"mutation {
                  updateOneTestModel(where: { id: 1 }, data: { json: null }) {
                    json
                  }
                }"#,
            2009,
            "A value is required but not set."
        );

        assert_error!(
            &runner,
            r#"mutation {
                updateOneTestModel(where: { id: 1 }, data: { json: AnyNull }) {
                  id
                }
              }"#,
            2009,
            "Enum value 'AnyNull' is invalid for enum type NullableJsonNullValueInput"
        );

        Ok(())
    }
}
