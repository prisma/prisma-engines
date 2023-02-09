use query_engine_tests::*;

#[test_suite(schema(schema), capabilities(Enums))]
mod create {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query, TROUBLE_CHARS};

    fn schema() -> String {
        let schema = indoc! {
            r#"model ScalarModel {
              #id(id, String, @id)
              optString   String?
              optInt      Int?
              optFloat    Float?
              optBoolean  Boolean?
              optEnum     MyEnum?
              optDateTime DateTime?
              optUnique   String? @unique
              createdAt   DateTime @default(now())
              relId       String?
              optRel      RelatedModel? @relation(fields: [relId], references: [id])
          }

          model RelatedModel {
            #id(id, String, @id)
              m  ScalarModel[]
          }

          enum MyEnum {
             A
             B
          }"#
        };

        schema.to_owned()
    }

    // "A Create Mutation" should "create and return item"
    #[connector_test]
    async fn create_should_work(runner: Runner) -> TestResult<()> {
        // This test is flaky on CockroachDB because of a TX write conflict.
        // We mitigate this issue by retrying multiple times.
        let res = retry!(
            {
                runner.query(format!(
                    r#"mutation {{
                      createOneScalarModel(data: {{
                        id: "1",
                        optString: "lala{TROUBLE_CHARS}",
                        optInt: 1337,
                        optFloat: 1.234,
                        optBoolean: true,
                        optEnum: A,
                        optDateTime: "2016-07-31T23:59:01.000Z"
                      }}) {{
                        id, optString, optInt, optFloat, optBoolean, optEnum, optDateTime
                      }}
                    }}"#
                ))
            },
            5
        );

        insta::assert_snapshot!(
          res.to_string(),
          @r###"{"data":{"createOneScalarModel":{"id":"1","optString":"lala¥฿😀😁😂😃😄😅😆😇😈😉😊😋😌😍😎😏😐😑😒😓😔😕😖😗😘😙😚😛😜😝😞😟😠😡😢😣😤😥😦😧😨😩😪😫😬😭😮😯😰😱😲😳😴😵😶😷😸😹😺😻😼😽😾😿🙀🙁🙂🙃🙄🙅🙆🙇🙈🙉🙊🙋🙌🙍🙎🙏ऀँंःऄअआइईउऊऋऌऍऎएऐऑऒओऔकखगघङचछजझञटठडढणतथदधनऩपफबभमयर€₭₮₯₰₱₲₳₴₵₶₷₸₹₺₻₼₽₾₿⃀","optInt":1337,"optFloat":1.234,"optBoolean":true,"optEnum":"A","optDateTime":"2016-07-31T23:59:01.000Z"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyScalarModel{ optString, optInt, optFloat, optBoolean, optEnum, optDateTime }}"#),
          @r###"{"data":{"findManyScalarModel":[{"optString":"lala¥฿😀😁😂😃😄😅😆😇😈😉😊😋😌😍😎😏😐😑😒😓😔😕😖😗😘😙😚😛😜😝😞😟😠😡😢😣😤😥😦😧😨😩😪😫😬😭😮😯😰😱😲😳😴😵😶😷😸😹😺😻😼😽😾😿🙀🙁🙂🙃🙄🙅🙆🙇🙈🙉🙊🙋🙌🙍🙎🙏ऀँंःऄअआइईउऊऋऌऍऎएऐऑऒओऔकखगघङचछजझञटठडढणतथदधनऩपफबभमयर€₭₮₯₰₱₲₳₴₵₶₷₸₹₺₻₼₽₾₿⃀","optInt":1337,"optFloat":1.234,"optBoolean":true,"optEnum":"A","optDateTime":"2016-07-31T23:59:01.000Z"}]}}"###
        );

        Ok(())
    }

    // "A Create Mutation" should "create and return item with empty string"
    #[connector_test]
    async fn return_item_empty_str(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneScalarModel(data: {
              id: "1",
              optString: ""
            }){
              optString, optInt, optFloat, optBoolean, optEnum
            }
          }"#),
          @r###"{"data":{"createOneScalarModel":{"optString":"","optInt":null,"optFloat":null,"optBoolean":null,"optEnum":null}}}"###
        );

        Ok(())
    }

    // A Create Mutation should create and return item with explicit null attributes
    // TODO: Flaky test on Cockroach, re-enable once figured out
    #[connector_test(exclude(CockroachDb))]
    async fn return_item_explicit_null_attrs(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneScalarModel(data: {
              id: "1",
              optString: null, optInt: null, optBoolean: null, optEnum: null, optFloat: null
            }){
              optString, optInt, optFloat, optBoolean, optEnum
            }
          }"#),
          @r###"{"data":{"createOneScalarModel":{"optString":null,"optInt":null,"optFloat":null,"optBoolean":null,"optEnum":null}}}"###
        );

        Ok(())
    }

    // "A Create Mutation" should "create and return item with implicit null attributes and createdAt should be set"
    #[connector_test]
    async fn return_item_implicit_null_attr(runner: Runner) -> TestResult<()> {
        // if the query succeeds createdAt did work. If would not have been set we would get a NullConstraintViolation.
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneScalarModel(data:{ id: "1" }){
              optString, optInt, optFloat, optBoolean, optEnum
            }
          }"#),
          @r###"{"data":{"createOneScalarModel":{"optString":null,"optInt":null,"optFloat":null,"optBoolean":null,"optEnum":null}}}"###
        );

        Ok(())
    }

    // A Create Mutation should create and return item with explicit null values after previous mutation with explicit non-null values
    #[connector_test(exclude(CockroachDb))]
    async fn return_item_non_null_attrs_then_explicit_null_attrs(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneScalarModel(
              data: { id: "1", optString: "lala", optInt: 123, optBoolean: true, optEnum: A, optFloat: 1.23}
            ) {
              optString, optInt, optFloat, optBoolean, optEnum
            }
           }"#),
          @r###"{"data":{"createOneScalarModel":{"optString":"lala","optInt":123,"optFloat":1.23,"optBoolean":true,"optEnum":"A"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneScalarModel(
              data: { id: "2", optString: null, optInt: null, optBoolean: null, optEnum: null, optFloat: null}
            ) {
              optString, optInt, optFloat, optBoolean, optEnum
            }
           }"#),
          @r###"{"data":{"createOneScalarModel":{"optString":null,"optInt":null,"optFloat":null,"optBoolean":null,"optEnum":null}}}"###
        );

        Ok(())
    }

    // "A Create Mutation" should "fail when a DateTime is invalid"
    #[connector_test]
    async fn fail_when_datetime_invalid(runner: Runner) -> TestResult<()> {
        assert_error!(runner, r#"mutation {
          createOneScalarModel(data:{
            id: "1",
            optString: "test",
            optInt: 1337,
            optFloat: 1.234,
            optBoolean: true,
            optEnum: A,
            optDateTime: "2016-0B-31T23:59:01.000Z"
          }) {
              optString, optInt, optFloat, optBoolean, optEnum, optDateTime
            }
          }"#,
          2009,
          "`Mutation.createOneScalarModel.data.ScalarModelCreateInput.optDateTime`: Error parsing value: Invalid DateTime: '2016-0B-31T23:59:01.000Z' (must be ISO 8601 compatible). Underlying error: input contains invalid characters."
        );

        Ok(())
    }

    // "A Create Mutation" should "fail when an Int is invalid"
    #[connector_test]
    async fn fail_when_int_invalid(runner: Runner) -> TestResult<()> {
        assert_error!(
          runner,
          r#"mutation {
            createOneScalarModel(data: {
              id: "1",
              optString: "test",
              optInt: B,
              optFloat: 1.234,
              optBoolean: true,
              optEnum: A,
              optDateTime: "2016-07-31T23:59:01.000Z"
            }
          ){ optString, optInt, optFloat, optBoolean, optEnum, optDateTime }}"#,
          2009,
          "Query parsing/validation error at `Mutation.createOneScalarModel.data.ScalarModelCreateInput.optInt`: Value types mismatch"
        );

        Ok(())
    }

    // "A Create Mutation" should "gracefully fail when a unique violation occurs"
    // TODO(dom): Not working on mongo
    // TODO(dom): 'Expected result to return an error, but found success: {"data":{"createOneScalarModel":{"optUnique":"test"}}}'
    // Comment(dom): Expected, we're not enforcing uniqueness for the test setup yet.
    #[connector_test(exclude(MongoDb))]
    async fn gracefully_fails_when_uniq_violation(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {createOneScalarModel(data: { id: "1", optUnique: "test"}){optUnique}}"#
        );

        assert_error!(
            &runner,
            r#"mutation {createOneScalarModel(data: { id: "2", optUnique: "test"}){optUnique}}"#,
            2002
        );
        Ok(())
    }

    // "A Create Mutation" should "create and return an item with enums passed as strings"
    // TODO: Flaky test on Cockroach, re-enable once figured out
    #[connector_test(exclude(CockroachDb))]
    async fn return_enums_passed_as_strings(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {createOneScalarModel(data: {id: "1", optEnum: "A"}){ optEnum }}"#),
          @r###"{"data":{"createOneScalarModel":{"optEnum":"A"}}}"###
        );

        Ok(())
    }

    // "A Create Mutation" should "fail if an item with enums passed as strings doesn't match and enum value"
    #[connector_test]
    async fn fail_if_string_dont_match_enum_val(runner: Runner) -> TestResult<()> {
        assert_error!(
          runner,
          r#"mutation {createOneScalarModel(data: {id: "1", optEnum: "NOPE"}){ optEnum }}"#,
          2009,
          "Query parsing/validation error at `Mutation.createOneScalarModel.data.ScalarModelCreateInput.optEnum`: Error parsing value: Enum value 'NOPE' is invalid for enum type MyEnum."
        );

        Ok(())
    }

    // "A Create Mutation" should "reject an optional relation set to null."
    #[connector_test]
    async fn reject_opt_rel_set_to_null(runner: Runner) -> TestResult<()> {
        assert_error!(
            &runner,
            r#"mutation { createOneScalarModel(data: { id: "1", optRel: null }){ relId }}"#,
            2009,
            "`Mutation.createOneScalarModel.data.ScalarModelCreateInput.optRel`: A value is required but not set"
        );

        Ok(())
    }

    // "A Create Mutation" should "create with an optional relation omitted."
    #[connector_test]
    async fn create_with_opt_rel_omitted(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneScalarModel(data: {id: "1"}) {
              relId
            }}"#),
          @r###"{"data":{"createOneScalarModel":{"relId":null}}}"###
        );

        Ok(())
    }

    fn schema_datetime() -> String {
        let schema = indoc! {
            r#"model A {
              #id(timestamp, DateTime, @id)
            }"#
        };

        schema.to_owned()
    }

    // "A Create Mutation with datetime as identifier" should "work"
    #[connector_test(schema(schema_datetime))]
    async fn create_with_datetime_ident(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneA(data: { timestamp: "1999-05-01T00:00:00.000Z" }) {
              timestamp
            }}"#),
          @r###"{"data":{"createOneA":{"timestamp":"1999-05-01T00:00:00.000Z"}}}"###
        );

        Ok(())
    }
}

#[test_suite(schema(json_opt), exclude(MySql(5.6)), capabilities(Json))]
mod json_create {
    use query_engine_tests::{assert_error, run_query};

    #[connector_test(only(MongoDb))]
    async fn create_json(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 1, json: "{}" }) { json }}"#),
          @r###"{"data":{"createOneTestModel":{"json":"{}"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 2, json: "null" }) { json }}"#),
          @r###"{"data":{"createOneTestModel":{"json":null}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 3, json: null }) { json }}"#),
          @r###"{"data":{"createOneTestModel":{"json":null}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 4 }) { json }}"#),
          @r###"{"data":{"createOneTestModel":{"json":null}}}"###
        );

        Ok(())
    }

    #[connector_test(capabilities(AdvancedJsonNullability))]
    async fn create_json_adv(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 1, json: "{}" }) { json }}"#),
          @r###"{"data":{"createOneTestModel":{"json":"{}"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 2, json: JsonNull }) { json }}"#),
          @r###"{"data":{"createOneTestModel":{"json":"null"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 3, json: DbNull }) { json }}"#),
          @r###"{"data":{"createOneTestModel":{"json":null}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 4 }) { json }}"#),
          @r###"{"data":{"createOneTestModel":{"json":null}}}"###
        );

        Ok(())
    }

    #[connector_test(capabilities(AdvancedJsonNullability))]
    async fn create_json_errors(runner: Runner) -> TestResult<()> {
        // On the JSON protocol, this succeeds because `null` is serialized as JSON.
        // It doesn't matter since the client does _not_ allow to send null values, but only DbNull or JsonNull.
        if runner.protocol().is_graphql() {
            assert_error!(
                &runner,
                r#"mutation {
                    createOneTestModel(data: { id: 1, json: null }) {
                      json
                    }
                  }"#,
                2009,
                "A value is required but not set."
            );
        }

        assert_error!(
            &runner,
            r#"mutation {
                createOneTestModel(data: { id: 1, json: AnyNull }) {
                  id
                }
              }"#,
            2009,
            "Enum value 'AnyNull' is invalid for enum type NullableJsonNullValueInput"
        );

        Ok(())
    }
}

#[test_suite(schema(schema_map))]
mod mapped_create {
    use query_engine_tests::run_query;
    fn schema_map() -> String {
        let schema = indoc! {
            r#"
            model GoodModel {
              #id(user_id, Int, @id)
              txt_space String @map("text space")
            }
            
            model AModel {
              #id(user_id, Int, @id @default(autoincrement()), @map("user id"))
              txt_space String @map("text space")
            }
            
            model BModel {
              #id(user_id, Int, @id, @map("user id"))
              txt_space String @map("text space")
            }
            
            model CModel {
              #id(user_id, String, @id, @map("user id"))
              txt_space String @map("text space")
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test(exclude(mongodb, cockroachdb))]
    async fn mapped_name_with_space_does_not_break_returning(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {createOneGoodModel(data: {user_id: 1, txt_space: "test"}) {user_id, txt_space}}"#),
          @r###"{"data":{"createOneGoodModel":{"user_id":1,"txt_space":"test"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {createOneAModel(data: {txt_space: "test"}) {user_id, txt_space}}"#),
          @r###"{"data":{"createOneAModel":{"user_id":1,"txt_space":"test"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {createOneBModel(data: {user_id: 1, txt_space: "test"}) {user_id, txt_space}}"#),
          @r###"{"data":{"createOneBModel":{"user_id":1,"txt_space":"test"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {createOneCModel(data: {user_id: "one", txt_space: "test"}) {user_id, txt_space}}"#),
          @r###"{"data":{"createOneCModel":{"user_id":"one","txt_space":"test"}}}"###
        );

        Ok(())
    }
}
