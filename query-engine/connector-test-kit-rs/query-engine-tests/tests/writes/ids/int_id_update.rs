use query_engine_tests::*;

#[test_suite]
mod int_id_update {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema_int() -> String {
        let schema = indoc! {
            r#"model Todo {
          #id(id, Int, @id)
          title String
        }"#
        };

        schema.to_owned()
    }

    // "Updating an item with an id field of type Int without default" should "work"
    #[connector_test(schema(schema_int))]
    async fn update_id_int_without_default(runner: Runner) -> TestResult<()> {
        // Setup
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneTodo(data: {title: "initial", id: 12}) {title, id}
          }"#),
          @r###"{"data":{"createOneTodo":{"title":"initial","id":12}}}"###
        );

        // Check
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTodo(where: {id: 12}, data: {title: {set: "the title"}}){
              id
              title
            }
          }"#),
          @r###"{"data":{"updateOneTodo":{"id":12,"title":"the title"}}}"###
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

    // "Updating an item with an id field of type Int with static default" should "work"
    #[connector_test(schema(schema_int_default))]
    async fn update_id_int_static_default(runner: Runner) -> TestResult<()> {
        // Setup
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneTodo(data: {title: "initial", id: 12}) {title, id}
          }"#),
          @r###"{"data":{"createOneTodo":{"title":"initial","id":12}}}"###
        );

        // Check
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTodo(where: {id: 12}, data: { title: { set: "the title" }}){
              id
              title
            }
          }"#),
          @r###"{"data":{"updateOneTodo":{"id":12,"title":"the title"}}}"###
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

    // "Updating an item with an id field of type Int with autoincrement" should "work"
    #[connector_test(schema(schema_int_autoinc), capabilities(AutoIncrement))]
    async fn update_id_int_autoinc(runner: Runner) -> TestResult<()> {
        // Setup
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneTodo(data: {title: "initial"}) {title, id}
          }"#),
          @r###"{"data":{"createOneTodo":{"title":"initial","id":1}}}"###
        );

        // Check
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTodo(where: {id: 1}, data: {title: {set: "the title"}}){
              id
              title
            }
          }"#),
          @r###"{"data":{"updateOneTodo":{"id":1,"title":"the title"}}}"###
        );

        Ok(())
    }

    fn schema_int_non_uniq_autoinc() -> String {
        let schema = indoc! {
            r#"model Todo {
              #id(id, String, @id)
              counter    Int @default(autoincrement())
              title      String
            }"#
        };

        schema.to_owned()
    }

    // "Updating a non-unique field of type Int with autoincrement" should "work"
    #[connector_test(
        schema(schema_int_non_uniq_autoinc),
        capabilities(AutoIncrement, AutoIncrementNonIndexedAllowed, WritableAutoincField)
    )]
    async fn update_non_uniq_int_field_autoinc(runner: Runner) -> TestResult<()> {
        // Setup
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneTodo(data: {id: "the-id", title: "initial"}) {title, id, counter}
          }"#),
          @r###"{"data":{"createOneTodo":{"title":"initial","id":"the-id","counter":1}}}"###
        );

        // Check
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneTodo(where: {id: "the-id"}, data: {title: { set: "the title" }, counter: { set: 8 }}){
              id
              title
              counter
            }
          }"#),
          @r###"{"data":{"updateOneTodo":{"id":"the-id","title":"the title","counter":8}}}"###
        );

        Ok(())
    }
}
