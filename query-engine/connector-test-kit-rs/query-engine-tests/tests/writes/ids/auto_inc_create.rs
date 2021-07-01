use query_engine_tests::*;

// TODO(dom): I assume it's expected that Mongo doens't have autoinc ids.
// I'm adding this todo just in case it's unexpected.
#[test_suite(schema(schema), capabilities(AutoIncrement))]
mod auto_inc_create {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema_1() -> String {
        let schema = indoc! {
            r#"model Mail {
              id Int   @default(autoincrement())
              #id(messageId, Int, @id)

              @@index(id)
          }"#
        };

        schema.to_owned()
    }

    // "Creating an item with a non primary key autoincrement and index " should "work"
    #[connector_test(schema(schema_1), exclude(Sqlite, MongoDb))]
    async fn non_primary_key_autoinc_idx(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneMail(data: { messageId:1 }){
              id
              messageId
            }
          }"#),
          @r###"{"data":{"createOneMail":{"id":1,"messageId":1}}}"###
        );

        Ok(())
    }

    fn schema_2() -> String {
        let schema = indoc! {
            r#"model Mail {
              id Int   @default(autoincrement()) @unique
              #id(messageId, Int, @id)
          }"#
        };

        schema.to_owned()
    }

    // "Creating an item with a non primary key autoincrement and unique index " should "work"
    #[connector_test(schema(schema_2), exclude(Sqlite, MongoDb))]
    async fn non_primary_key_autoinc_uniq_idx(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneMail(data: { messageId:1 }){
              id
              messageId
            }
          }"#),
          @r###"{"data":{"createOneMail":{"id":1,"messageId":1}}}"###
        );

        Ok(())
    }

    fn schema_3() -> String {
        let schema = indoc! {
            r#"model Mail {
              id Int   @default(autoincrement())
              #id(messageId, Int, @id)
          }"#
        };

        schema.to_owned()
    }

    // "Creating an item with a non primary key autoincrement without indexes" should "work"
    #[connector_test(schema(schema_3), exclude(Sqlite, Mysql, MongoDb))]
    async fn non_primary_key_autoinc_without_idx(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneMail(data: { messageId:1 }){
              id
              messageId
            }
          }"#),
          @r###"{"data":{"createOneMail":{"id":1,"messageId":1}}}"###
        );

        Ok(())
    }
}
