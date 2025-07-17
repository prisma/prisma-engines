use query_engine_tests::*;

#[test_suite(schema(schema), capabilities(ScalarLists))]
mod basic_types {
    use indoc::indoc;
    use query_engine_tests::{TROUBLE_CHARS, assert_error, run_query};

    fn schema() -> String {
        let schema = indoc! {
            r#"model ScalarModel {
              #id(id, Int, @id)
              strings   String[]
              ints      Int[]
              floats    Float[]
              booleans  Boolean[]
              enums     MyEnum[]
              dateTimes DateTime[]
              bytes     Bytes[]
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
    async fn set_base(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, format!(r#"mutation {{
            createOneScalarModel(data: {{
              id: 1,
              strings:   {{ set: ["test{TROUBLE_CHARS}"] }}
              ints:      {{ set: [1337, 12] }}
              floats:    {{ set: [1.234, 1.45] }}
              booleans:  {{ set: [true, false] }}
              enums:     {{ set: [A, A] }}
              dateTimes: {{ set: ["2016-07-31T23:59:01.000Z","2017-07-31T23:59:01.000Z"] }}
              bytes:     {{ set: ["dGVzdA==", "dA=="] }}
            }}) {{
              strings
              ints
              floats
              booleans
              enums
              dateTimes
              bytes
            }}
          }}"#)),
          @r###"{"data":{"createOneScalarModel":{"strings":["test¥฿😀😁😂😃😄😅😆😇😈😉😊😋😌😍😎😏😐😑😒😓😔😕😖😗😘😙😚😛😜😝😞😟😠😡😢😣😤😥😦😧😨😩😪😫😬😭😮😯😰😱😲😳😴😵😶😷😸😹😺😻😼😽😾😿🙀🙁🙂🙃🙄🙅🙆🙇🙈🙉🙊🙋🙌🙍🙎🙏ऀँंःऄअआइईउऊऋऌऍऎएऐऑऒओऔकखगघङचछजझञटठडढणतथदधनऩपफबभमयर€₭₮₯₰₱₲₳₴₵₶₷₸₹₺₻₼₽₾₿⃀"],"ints":[1337,12],"floats":[1.234,1.45],"booleans":[true,false],"enums":["A","A"],"dateTimes":["2016-07-31T23:59:01.000Z","2017-07-31T23:59:01.000Z"],"bytes":["dGVzdA==","dA=="]}}}"###
        );

        Ok(())
    }

    // "Scalar lists" should "be behave like regular values for create and update operations"
    // Skipped for CockroachDB as enum array concatenation is not supported (https://github.com/cockroachdb/cockroach/issues/71388).
    #[connector_test(exclude(CockroachDb))]
    async fn behave_like_regular_val_for_create_and_update(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, format!(r#"mutation {{
            createOneScalarModel(data: {{
              id: 1,
              strings:   {{ set: ["test{TROUBLE_CHARS}"] }}
              ints:      {{ set: [1337, 12] }}
              floats:    {{ set: [1.234, 1.45] }}
              booleans:  {{ set: [true, false] }}
              enums:     {{ set: [A, A] }}
              dateTimes: {{ set: ["2016-07-31T23:59:01.000Z","2017-07-31T23:59:01.000Z"] }}
              bytes:     {{ set: ["dGVzdA==", "dA=="] }}
            }}) {{
              strings
              ints
              floats
              booleans
              enums
              dateTimes
              bytes
            }}
          }}"#)),
          @r###"{"data":{"createOneScalarModel":{"strings":["test¥฿😀😁😂😃😄😅😆😇😈😉😊😋😌😍😎😏😐😑😒😓😔😕😖😗😘😙😚😛😜😝😞😟😠😡😢😣😤😥😦😧😨😩😪😫😬😭😮😯😰😱😲😳😴😵😶😷😸😹😺😻😼😽😾😿🙀🙁🙂🙃🙄🙅🙆🙇🙈🙉🙊🙋🙌🙍🙎🙏ऀँंःऄअआइईउऊऋऌऍऎएऐऑऒओऔकखगघङचछजझञटठडढणतथदधनऩपफबभमयर€₭₮₯₰₱₲₳₴₵₶₷₸₹₺₻₼₽₾₿⃀"],"ints":[1337,12],"floats":[1.234,1.45],"booleans":[true,false],"enums":["A","A"],"dateTimes":["2016-07-31T23:59:01.000Z","2017-07-31T23:59:01.000Z"],"bytes":["dGVzdA==","dA=="]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneScalarModel(where: { id: 1 }, data: {
              strings:   { set: ["updated", "now"] }
              ints:      { set: [14] }
              floats:    { set: [1.2345678] }
              booleans:  { set: [false, false, true] }
              enums:     { set: [] }
              dateTimes: { set: ["2019-07-31T23:59:01.000Z"] }
              bytes:     { set: ["dGVzdA=="] }
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
          @r###"{"data":{"updateOneScalarModel":{"strings":["updated","now"],"ints":[14],"floats":[1.2345678],"booleans":[false,false,true],"enums":[],"dateTimes":["2019-07-31T23:59:01.000Z"],"bytes":["dGVzdA=="]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneScalarModel(where: { id: 1 }, data: {
              strings:   { push: "future" }
              ints:      { push: 15 }
              floats:    { push: 2 }
              booleans:  { push: true }
              enums:     { push: A }
              dateTimes: { push: "2019-07-31T23:59:01.000Z" }
              bytes:     { push: "dGVzdA==" }
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
          @r###"{"data":{"updateOneScalarModel":{"strings":["updated","now","future"],"ints":[14,15],"floats":[1.2345678,2],"booleans":[false,false,true,true],"enums":["A"],"dateTimes":["2019-07-31T23:59:01.000Z","2019-07-31T23:59:01.000Z"],"bytes":["dGVzdA==","dGVzdA=="]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneScalarModel(where: { id: 1 }, data: {
              strings:   { push: ["more", "items"] }
              ints:      { push: [16, 17] }
              floats:    { push: [3, 4] }
              booleans:  { push: [false, true] }
              enums:     { push: [B, A] }
              dateTimes: { push: ["2019-07-31T23:59:01.000Z", "2019-07-31T23:59:01.000Z"] }
              bytes:     { push: ["dGVzdA==", "dGVzdA=="] }
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
          @r###"{"data":{"updateOneScalarModel":{"strings":["updated","now","future","more","items"],"ints":[14,15,16,17],"floats":[1.2345678,2,3,4],"booleans":[false,false,true,true,false,true],"enums":["A","B","A"],"dateTimes":["2019-07-31T23:59:01.000Z","2019-07-31T23:59:01.000Z","2019-07-31T23:59:01.000Z","2019-07-31T23:59:01.000Z"],"bytes":["dGVzdA==","dGVzdA==","dGVzdA==","dGVzdA=="]}}}"###
        );

        Ok(())
    }

    // "A Create Mutation" should "create and return items with list values with shorthand notation"
    #[connector_test]
    async fn create_mut_work_with_list_vals(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, format!(r#"mutation {{
            createOneScalarModel(data: {{
              id: 1
              strings:   ["test{TROUBLE_CHARS}"]
              ints:      [1337, 12]
              floats:    [1.234, 1.45]
              booleans:  [true, false]
              enums:     [A,A]
              dateTimes: ["2016-07-31T23:59:01.000Z","2017-07-31T23:59:01.000Z"]
              bytes:     ["dGVzdA==", "dA=="]
            }}) {{
              strings
              ints
              floats
              booleans
              enums
              dateTimes
              bytes
            }}
          }}"#)),
          @r###"{"data":{"createOneScalarModel":{"strings":["test¥฿😀😁😂😃😄😅😆😇😈😉😊😋😌😍😎😏😐😑😒😓😔😕😖😗😘😙😚😛😜😝😞😟😠😡😢😣😤😥😦😧😨😩😪😫😬😭😮😯😰😱😲😳😴😵😶😷😸😹😺😻😼😽😾😿🙀🙁🙂🙃🙄🙅🙆🙇🙈🙉🙊🙋🙌🙍🙎🙏ऀँंःऄअआइईउऊऋऌऍऎएऐऑऒओऔकखगघङचछजझञटठडढणतथदधनऩपफबभमयर€₭₮₯₰₱₲₳₴₵₶₷₸₹₺₻₼₽₾₿⃀"],"ints":[1337,12],"floats":[1.234,1.45],"booleans":[true,false],"enums":["A","A"],"dateTimes":["2016-07-31T23:59:01.000Z","2017-07-31T23:59:01.000Z"],"bytes":["dGVzdA==","dA=="]}}}"###
        );

        Ok(())
    }

    // "A Create Mutation" should "create and return items with empty list values"
    #[connector_test]
    async fn create_mut_return_items_with_empty_lists(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneScalarModel(data: {
              id: 1
              strings:   []
              ints:      []
              floats:    []
              booleans:  []
              enums:     []
              dateTimes: []
              bytes:     []
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

    // "A Create Mutation with an empty scalar list create input object" should "return a detailed error"
    #[connector_test]
    async fn create_mut_empty_scalar_should_fail(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"mutation {
            createOneScalarModel(data: {
              id: 1
              strings: {},
            }){ strings, ints, floats, booleans, enums, dateTimes }
          }"#,
            2009,
            "A value is required but not set"
        );

        Ok(())
    }

    // "An Update Mutation with an empty scalar list update input object" should "return a detailed error"
    #[connector_test]
    async fn update_mut_empty_scalar_should_fail(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"mutation {
            updateOneScalarModel(data: {
              strings: {},
            }){ strings, ints, floats, booleans, enums, dateTimes }
          }"#,
            2009,
            "Some fields are missing: Expected exactly one field to be present, got 0."
        );

        Ok(())
    }

    // Skipped for CockroachDB as enum array concatenation is not supported (https://github.com/cockroachdb/cockroach/issues/71388).
    #[connector_test(exclude(CockroachDb))]
    async fn update_mut_push_empty_enum_array(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1 }"#).await?;
        create_row(&runner, r#"{ id: 2 }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneScalarModel(where: { id: 1 }, data: {
              enums:     { push: A }
            }) {
              enums
            }
          }"#),
          @r###"{"data":{"updateOneScalarModel":{"enums":["A"]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneScalarModel(where: { id: 2 }, data: {
              enums:     { push: [A, B] }
            }) {
              enums
            }
          }"#),
          @r###"{"data":{"updateOneScalarModel":{"enums":["A","B"]}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn update_mut_push_empty_scalar_list(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1 }"#).await?;
        create_row(&runner, r#"{ id: 2 }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneScalarModel(where: { id: 1 }, data: {
              strings:   { push: "future" }
              ints:      { push: 15 }
              floats:    { push: 2 }
              booleans:  { push: true }
              dateTimes: { push: "2019-07-31T23:59:01.000Z" }
              bytes:     { push: "dGVzdA==" }
            }) {
              strings
              ints
              floats
              booleans
              dateTimes
              bytes
            }
          }"#),
          @r###"{"data":{"updateOneScalarModel":{"strings":["future"],"ints":[15],"floats":[2],"booleans":[true],"dateTimes":["2019-07-31T23:59:01.000Z"],"bytes":["dGVzdA=="]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneScalarModel(where: { id: 2 }, data: {
              strings:   { push: ["present", "future"] }
              ints:      { push: [14, 15] }
              floats:    { push: [1, 2] }
              booleans:  { push: [false, true] }
              dateTimes: { push: ["2019-07-31T23:59:01.000Z", "2019-07-31T23:59:02.000Z"] }
              bytes:     { push: ["dGVzdA==", "dGVzdA=="] }
            }) {
              strings
              ints
              floats
              booleans
              dateTimes
              bytes
            }
          }"#),
          @r###"{"data":{"updateOneScalarModel":{"strings":["present","future"],"ints":[14,15],"floats":[1,2],"booleans":[false,true],"dateTimes":["2019-07-31T23:59:01.000Z","2019-07-31T23:59:02.000Z"],"bytes":["dGVzdA==","dGVzdA=="]}}}"###
        );

        Ok(())
    }

    // Test that Cockroach will not work with enum push
    #[connector_test(only(CockroachDb))]
    async fn cockroachdb_doesnot_support_enum_push(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1 }"#).await?;
        create_row(&runner, r#"{ id: 2 }"#).await?;

        assert_error!(
            &runner,
            r#"mutation { updateOneScalarModel(where: { id: 1 }, data: { enums: { push: A }}) { id }}"#,
            2009,
            "Unable to match input value to any allowed input type for the field"
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneScalarModel(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}
