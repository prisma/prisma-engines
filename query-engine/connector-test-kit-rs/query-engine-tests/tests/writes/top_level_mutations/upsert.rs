use query_engine_tests::*;

#[test_suite(schema(schema))]
mod upsert {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query, run_query_json};

    fn schema() -> String {
        let schema = indoc! {
            r#"model Todo {
              #id(id, Int, @id)
              title          String
              alias          String  @unique
              anotherIDField String? @unique
            }

            model WithDefaultValue {
              #id(id, Int, @id)
              reqString String @default(value: "defaultValue")
              title     String
            }

            model MultipleFields {
              #id(id, Int, @id)
              reqString  String
              reqInt     Int
              reqFloat   Float
              reqBoolean Boolean
            }"#
        };

        schema.to_owned()
    }

    // "an item" should "be created if it does not exist yet"
    #[connector_test]
    async fn item_should_be_upserted(runner: Runner) -> TestResult<()> {
        assert_eq!(count_todo(&runner).await?, 0);

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            upsertOneTodo(
              where: {id: 1}
              create: {
                id: 1,
                title: "new title"
                alias: "todo1"
              }
              update: {
                title: { set: "updated title" }
              }
            ){
              id
              title
            }
          }"#),
          @r###"{"data":{"upsertOneTodo":{"id":1,"title":"new title"}}}"###
        );

        assert_eq!(count_todo(&runner).await?, 1);

        Ok(())
    }

    // "an item" should "be created with multiple fields of different types"
    #[connector_test]
    async fn create_with_many_fields_of_diff_types(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            upsertOneMultipleFields(
              where: {id: 1}
              create: {
                id: 1,
                reqString: "new title"
                reqInt: 1
                reqFloat: 1.22
                reqBoolean: true
              }
              update: {
                reqString: { set: "title" }
                reqInt: { set: 2 }
                reqFloat: { set: 5.223423423423 }
                reqBoolean: { set: false }
              }
            ){
              id
              reqString
              reqInt
              reqFloat
              reqBoolean
            }
          }"#),
          @r###"{"data":{"upsertOneMultipleFields":{"id":1,"reqString":"new title","reqInt":1,"reqFloat":1.22,"reqBoolean":true}}}"###
        );

        Ok(())
    }

    // "an item" should "be created if it does not exist yet and use the defaultValue if necessary"
    #[connector_test]
    async fn create_if_not_exist_with_default_val(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            upsertOneWithDefaultValue(
              where: {id: 1}
              create: {
                id: 1,
                title: "new title"
              }
              update: {
                title: { set: "updated title" }
              }
            ){
              title
              reqString
            }
          }"#),
          @r###"{"data":{"upsertOneWithDefaultValue":{"title":"new title","reqString":"defaultValue"}}}"###
        );

        Ok(())
    }

    // "an item" should "not be created when trying to set a required value to null even if there is a default value for that field"
    #[connector_test]
    async fn no_create_required_val_null(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            r#"mutation {
            upsertOneWithDefaultValue(
              where: {id: 1337}
              create: {
                id: 1,
                reqString: null
                title: "new title"
              }
              update: {
                title: { set: "updated title" }
              }
            ){
              title
              reqString
            }
          }"#,
            2009,
            "`create.reqString`: A value is required but not set"
        );

        Ok(())
    }

    // "an item" should "be updated if it already exists (by id)"
    #[connector_test]
    async fn update_if_already_exists(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
                createOneTodo(
                  data: {
                    id: 1,
                    title: "new title1"
                    alias: "todo1"
                  }
                ) {
                  id
                }
            }"#
        );

        assert_eq!(count_todo(&runner).await?, 1);

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            upsertOneTodo(
              where: {id: 1}
              create: {
                id: 1,
                title: "irrelevant"
                alias: "irrelevant"
              }
              update: {
                title: { set: "updated title" }
              }
            ){
              id
              title
            }
          }"#),
          @r###"{"data":{"upsertOneTodo":{"id":1,"title":"updated title"}}}"###
        );

        assert_eq!(count_todo(&runner).await?, 1);

        Ok(())
    }

    // "an item" should "be updated using shorthands if it already exists (by id)"
    #[connector_test]
    async fn update_shorthand_already_exists(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
                createOneTodo(
                  data: {
                    id: 1,
                    title: "new title1"
                    alias: "todo1"
                  }
                ) {
                id
              }
          }"#
        );

        assert_eq!(count_todo(&runner).await?, 1);

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            upsertOneTodo(
              where: {id: 1}
              create: {
                id: 1,
                title: "irrelevant"
                alias: "irrelevant"
              }
              update: {
                title: "updated title"
              }
            ){
              id
              title
            }
          }"#),
          @r###"{"data":{"upsertOneTodo":{"id":1,"title":"updated title"}}}"###
        );

        assert_eq!(count_todo(&runner).await?, 1);

        Ok(())
    }

    // "an item" should "be updated if it already exists (by any unique argument)"
    #[connector_test]
    async fn should_update_if_uniq_already_exists(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
                createOneTodo(
                  data: {
                    id: 1,
                    title: "new title1"
                    alias: "todo1"
                  }
                ) {
                  alias
                }
            }"#
        );

        assert_eq!(count_todo(&runner).await?, 1);

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            upsertOneTodo(
              where: {alias: "todo1"}
              create: {
                id: 1,
                title: "irrelevant"
                alias: "irrelevant"
              }
              update: {
                title: { set:"updated title" }
              }
            ){
              id
              title
            }
          }"#),
          @r###"{"data":{"upsertOneTodo":{"id":1,"title":"updated title"}}}"###
        );

        assert_eq!(count_todo(&runner).await?, 1);

        Ok(())
    }

    // "An upsert" should "perform only an update if the update changes the unique field used in the where clause"
    #[connector_test]
    async fn only_update_if_uniq_field_change(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
                createOneTodo(
                  data: {
                    id: 1,
                    title: "title"
                    alias: "todo1"
                  }
                ) {
                  id
                }
            }"#
        );

        assert_eq!(count_todo(&runner).await?, 1);

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            upsertOneTodo(
              where: {alias: "todo1"}
              create: {
                id: 1,
                title: "title of new node"
                alias: "alias-of-new-node"
              }
              update: {
                title: { set: "updated title" }
                alias: { set:"todo1-new" }
              }
            ){
              id
              title
            }
          }"#),
          @r###"{"data":{"upsertOneTodo":{"id":1,"title":"updated title"}}}"###
        );

        assert_eq!(count_todo(&runner).await?, 1);

        // the original node has been updated
        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findUniqueTodo(where: {id: 1}){
              title
            }
          }"#),
          @r###"{"data":{"findUniqueTodo":{"title":"updated title"}}}"###
        );

        Ok(())
    }

    // "An upsert" should "perform only an update if the update changes nothing"
    #[connector_test]
    async fn only_update_if_update_changes_nothing(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
                createOneTodo(
                  data: {
                    id: 1,
                    title: "title"
                    alias: "todo1"
                  }
                ) {
                  id
                }
            }"#
        );

        assert_eq!(count_todo(&runner).await?, 1);

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            upsertOneTodo(
              where: {alias: "todo1"}
              create: {
                id: 1,
                title: "title of new node"
                alias: "alias-of-new-node"
              }
              update: {
                title: { set: "title" }
                alias: { set: "todo1" }
              }
            ){
              id
              title
            }
          }"#),
          @r###"{"data":{"upsertOneTodo":{"id":1,"title":"title"}}}"###
        );

        assert_eq!(count_todo(&runner).await?, 1);

        // the original node has been updated
        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findUniqueTodo(where: {id: 1}){
              title
            }
          }"#),
          @r###"{"data":{"findUniqueTodo":{"title":"title"}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(schema), relation_mode = "prisma")]
    async fn upsert_called_twice_does_nothing(runner: Runner) -> TestResult<()> {
        assert_eq!(count_todo(&runner).await?, 0);

        const MUTATION: &str = r#"mutation {
            upsertOneTodo(
                where: {id: 1}
                create: {
                    id: 1,
                    title: "title"
                    alias: "alias"
                }
                update: {
                title: { set: "title" }
                }
            ){
                id
              title
            }
        }"#;

        insta::assert_snapshot!(run_query!(&runner, MUTATION), @r#"{"data":{"upsertOneTodo":{"id":1,"title":"title"}}}"#);
        insta::assert_snapshot!(run_query!(&runner, MUTATION), @r#"{"data":{"upsertOneTodo":{"id":1,"title":"title"}}}"#);

        assert_eq!(count_todo(&runner).await?, 1);

        Ok(())
    }

    fn schema_number() -> String {
        let schema = indoc! {
            r#"model TestModel {
            #id(id, Int, @id)
            optInt   Int?
            optFloat Float?
          }"#
        };

        schema.to_owned()
    }

    // "An upsert mutation" should "correctly apply all number operations for Int"
    #[connector_test(schema(schema_number), exclude(CockroachDb))]
    async fn upsert_apply_number_ops_for_int(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1 }"#).await?;
        create_row(&runner, r#"{ id: 2, optInt: 3}"#).await?;

        // Increment
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optInt", "increment", "10").await?,
          @r###"{"data":{"upsertOneTestModel":{"optInt":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optInt", "increment", "10").await?,
          @r###"{"data":{"upsertOneTestModel":{"optInt":13}}}"###
        );

        // Decrement
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optInt", "decrement", "10").await?,
          @r###"{"data":{"upsertOneTestModel":{"optInt":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optInt", "decrement", "10").await?,
          @r###"{"data":{"upsertOneTestModel":{"optInt":3}}}"###
        );

        // Multiply
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optInt", "multiply", "2").await?,
          @r###"{"data":{"upsertOneTestModel":{"optInt":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optInt", "multiply", "2").await?,
          @r###"{"data":{"upsertOneTestModel":{"optInt":6}}}"###
        );

        // Divide
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optInt", "divide", "3").await?,
          @r###"{"data":{"upsertOneTestModel":{"optInt":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optInt", "divide", "3").await?,
          @r###"{"data":{"upsertOneTestModel":{"optInt":2}}}"###
        );

        // Set
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optInt", "set", "5").await?,
          @r###"{"data":{"upsertOneTestModel":{"optInt":5}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optInt", "set", "5").await?,
          @r###"{"data":{"upsertOneTestModel":{"optInt":5}}}"###
        );

        // Set null
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optInt", "set", "null").await?,
          @r###"{"data":{"upsertOneTestModel":{"optInt":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optInt", "set", "null").await?,
          @r###"{"data":{"upsertOneTestModel":{"optInt":null}}}"###
        );

        Ok(())
    }

    // CockroachDB does not support the "divide" operator as is.
    // See https://github.com/cockroachdb/cockroach/issues/41448.
    #[connector_test(schema(schema_number), only(CockroachDb))]
    async fn upsert_apply_number_ops_for_int_cockroach(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1 }"#).await?;
        create_row(&runner, r#"{ id: 2, optInt: 3}"#).await?;

        // Increment
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optInt", "increment", "10").await?,
          @r###"{"data":{"upsertOneTestModel":{"optInt":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optInt", "increment", "10").await?,
          @r###"{"data":{"upsertOneTestModel":{"optInt":13}}}"###
        );

        // Decrement
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optInt", "decrement", "10").await?,
          @r###"{"data":{"upsertOneTestModel":{"optInt":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optInt", "decrement", "10").await?,
          @r###"{"data":{"upsertOneTestModel":{"optInt":3}}}"###
        );

        // Multiply
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optInt", "multiply", "2").await?,
          @r###"{"data":{"upsertOneTestModel":{"optInt":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optInt", "multiply", "2").await?,
          @r###"{"data":{"upsertOneTestModel":{"optInt":6}}}"###
        );

        // Set
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optInt", "set", "5").await?,
          @r###"{"data":{"upsertOneTestModel":{"optInt":5}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optInt", "set", "5").await?,
          @r###"{"data":{"upsertOneTestModel":{"optInt":5}}}"###
        );

        // Set null
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optInt", "set", "null").await?,
          @r###"{"data":{"upsertOneTestModel":{"optInt":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optInt", "set", "null").await?,
          @r###"{"data":{"upsertOneTestModel":{"optInt":null}}}"###
        );

        Ok(())
    }

    // "An upsert mutation" should "correctly apply all number operations for Float"
    #[connector_test(schema(schema_number), exclude(MongoDb))]
    async fn upsert_apply_number_ops_for_float(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1 }"#).await?;
        create_row(&runner, r#"{ id: 2, optFloat: 5.5}"#).await?;

        // Increment
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optFloat", "increment", "4.6").await?,
          @r###"{"data":{"upsertOneTestModel":{"optFloat":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optFloat", "increment", "4.6").await?,
          @r###"{"data":{"upsertOneTestModel":{"optFloat":10.1}}}"###
        );

        // Decrement
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optFloat", "decrement", "4.6").await?,
          @r###"{"data":{"upsertOneTestModel":{"optFloat":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optFloat", "decrement", "4.6").await?,
          @r###"{"data":{"upsertOneTestModel":{"optFloat":5.5}}}"###
        );

        // Multiply
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optFloat", "multiply", "2").await?,
          @r###"{"data":{"upsertOneTestModel":{"optFloat":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optFloat", "multiply", "2").await?,
          @r###"{"data":{"upsertOneTestModel":{"optFloat":11.0}}}"###
        );

        // Divide
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optFloat", "divide", "2").await?,
          @r###"{"data":{"upsertOneTestModel":{"optFloat":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optFloat", "divide", "2").await?,
          @r###"{"data":{"upsertOneTestModel":{"optFloat":5.5}}}"###
        );

        // Set
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optFloat", "set", "5.1").await?,
          @r###"{"data":{"upsertOneTestModel":{"optFloat":5.1}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optFloat", "set", "5.1").await?,
          @r###"{"data":{"upsertOneTestModel":{"optFloat":5.1}}}"###
        );

        // Set null
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optFloat", "set", "null").await?,
          @r###"{"data":{"upsertOneTestModel":{"optFloat":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optFloat", "set", "null").await?,
          @r###"{"data":{"upsertOneTestModel":{"optFloat":null}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(schema_number), only(MongoDb))]
    async fn upsert_apply_number_ops_for_float_mongo(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1 }"#).await?;
        create_row(&runner, r#"{ id: 2, optFloat: 5.5}"#).await?;

        // Increment
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optFloat", "increment", "4.6").await?,
          @r###"{"data":{"upsertOneTestModel":{"optFloat":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optFloat", "increment", "4.6").await?,
          @r###"{"data":{"upsertOneTestModel":{"optFloat":10.1}}}"###
        );

        // Decrement
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optFloat", "decrement", "4.6").await?,
          @r###"{"data":{"upsertOneTestModel":{"optFloat":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optFloat", "decrement", "4.6").await?,
          @r###"{"data":{"upsertOneTestModel":{"optFloat":5.5}}}"###
        );

        // Multiply
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optFloat", "multiply", "2").await?,
          @r###"{"data":{"upsertOneTestModel":{"optFloat":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optFloat", "multiply", "2").await?,
          @r###"{"data":{"upsertOneTestModel":{"optFloat":11.0}}}"###
        );

        // Divide
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optFloat", "divide", "2").await?,
          @r###"{"data":{"upsertOneTestModel":{"optFloat":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optFloat", "divide", "2").await?,
          @r###"{"data":{"upsertOneTestModel":{"optFloat":5.5}}}"###
        );

        // Set
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optFloat", "set", "5.1").await?,
          @r###"{"data":{"upsertOneTestModel":{"optFloat":5.1}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optFloat", "set", "5.1").await?,
          @r###"{"data":{"upsertOneTestModel":{"optFloat":5.1}}}"###
        );

        // Set null
        insta::assert_snapshot!(
          query_number_operation(&runner, "1", "optFloat", "set", "null").await?,
          @r###"{"data":{"upsertOneTestModel":{"optFloat":null}}}"###
        );
        insta::assert_snapshot!(
          query_number_operation(&runner, "2", "optFloat", "set", "null").await?,
          @r###"{"data":{"upsertOneTestModel":{"optFloat":null}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(generic))]
    async fn upsert_fails_if_filter_dont_match(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation { createOneTestModel(data: { id: 1, field: "hello" }) { id } }"#
        );

        assert_error!(
            &runner,
            r#"mutation {
                  upsertOneTestModel(where: { id: 1, field: "bonjour" }, create: { id: 1, field: "hello" }, update: { field: "updated" }) {
                    id
                  }
                }"#,
            2002,
            "Unique constraint failed"
        );

        Ok(())
    }

    async fn count_todo(runner: &Runner) -> TestResult<usize> {
        let res = run_query_json!(runner, r#"{ findManyTodo { id } }"#);
        let data = &res["data"]["findManyTodo"];

        match data {
            serde_json::Value::Array(arr) => Ok(arr.len()),
            _ => unreachable!(),
        }
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
            upsertOneTestModel(
              where: {{ id: {id} }}
              create: {{ id: {id} }}
              update: {{ {field}: {{ {op}: {value} }} }}
            ){{
              {field}
            }}
          }}"#
            )
        );

        Ok(res)
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTestModel(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}
