use query_engine_tests::*;

#[test_suite]
mod int_id_create {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query};

    fn schema_int() -> String {
        let schema = indoc! {
            r#"model Todo {
              #id(id, Int, @id)
              title String
            }"#
        };

        schema.to_owned()
    }

    // "Creating an item with an id field of type Int without default"
    #[connector_test(schema(schema_int))]
    async fn create_id_int_without_default(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneTodo(data: { title: "the title", id: 10 }){
              id
              title
            }
          }"#),
          @r###"{"data":{"createOneTodo":{"id":10,"title":"the title"}}}"###
        );

        Ok(())
    }

    // "Creating an item with an id field of type Int without default without providing the id"
    #[connector_test(schema(schema_int))]
    async fn create_id_int_without_default_without_id(runner: Runner) -> TestResult<()> {
        assert_error!(
            &runner,
            r#"mutation {
              createOneTodo(data: { title: "the title" }){
                id
                title
              }
            }"#,
            2009
        );

        Ok(())
    }

    fn schema_int_default() -> String {
        let schema = indoc! {
            r#"model Todo {
              #id(id, Int, @id, @default(0))
              title String
            }"#
        };

        schema.to_owned()
    }

    // "Creating an item with an id field of type Int with static default"
    #[connector_test(schema(schema_int_default))]
    async fn create_id_int_static_default(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneTodo(data: { title: "the title", id: 10 }){
              id
              title
            }
          }"#),
          @r###"{"data":{"createOneTodo":{"id":10,"title":"the title"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneTodo(data: { title: "the title"}){
              id
              title
            }
          }"#),
          @r###"{"data":{"createOneTodo":{"id":0,"title":"the title"}}}"###
        );

        Ok(())
    }

    fn schema_int_autoinc() -> String {
        let schema = indoc! {
            r#"model Todo {
              #id(id, Int, @id, @default(autoincrement()))
              title String
            }"#
        };

        schema.to_owned()
    }

    // "Creating an item with an id field of type Int with autoincrement" should "work"
    #[connector_test(schema(schema_int_autoinc), capabilities(AutoIncrement))]
    async fn create_id_int_with_autoinc(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneTodo(data: { title: "the title"}){
              id
              title
            }
          }"#),
          @r###"{"data":{"createOneTodo":{"id":1,"title":"the title"}}}"###
        );

        Ok(())
    }

    fn schema_int_autoinc_provide_id() -> String {
        let schema = indoc! {
            r#"model A {
              #id(id, Int, @id, @default(autoincrement()))
              b_id Int
              b    B   @relation(fields: [b_id], references: [id])
            }

            model B {
              #id(id, Int, @id)
              a  A[]
            }"#
        };

        schema.to_owned()
    }

    // "Creating an item with an id field of type Int with autoincrement and providing an id" should "error for checked inputs"
    #[connector_test(schema(schema_int_autoinc_provide_id), capabilities(AutoIncrement))]
    async fn create_id_int_autoinc_providing_id(runner: Runner) -> TestResult<()> {
        assert_error!(
            &runner,
            r#"mutation {
              createOneA(data: { id: 2, b: { connect: { id: 1 }}}) {
                id
                b { id }
              }
            }"#,
            2009
        );

        Ok(())
    }
}
