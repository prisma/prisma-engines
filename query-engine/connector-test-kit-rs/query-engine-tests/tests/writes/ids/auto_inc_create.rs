use query_engine_tests::*;

#[test_suite(schema(schema))]
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
    #[connector_test(schema(schema_1), capabilities(AutoIncrement, AutoIncrementNonIndexedAllowed))]
    async fn non_primary_key_autoinc_idx(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
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
              #id(messageId, Int, @id)
              id Int @default(autoincrement()) @unique
          }"#
        };

        schema.to_owned()
    }

    // "Creating an item with a non primary key autoincrement and unique index " should "work"
    #[connector_test(schema(schema_2), capabilities(AutoIncrement, AutoIncrementAllowedOnNonId))]
    async fn non_primary_key_autoinc_uniq_idx(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
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
              #id(messageId, Int, @id)
              id Int   @default(autoincrement())
          }"#
        };

        schema.to_owned()
    }

    // "Creating an item with a non primary key autoincrement without indexes" should "work"
    #[connector_test(
        schema(schema_3),
        capabilities(AutoIncrement, AutoIncrementNonIndexedAllowed, AutoIncrementAllowedOnNonId)
    )]
    async fn non_primary_key_autoinc_without_idx(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneMail(data: { messageId: 1 }){
              id
              messageId
            }
          }"#),
          @r###"{"data":{"createOneMail":{"id":1,"messageId":1}}}"###
        );

        Ok(())
    }
}
