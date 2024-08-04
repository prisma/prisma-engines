use query_engine_tests::*;

#[test_suite(schema(schema))]
mod compound_fks {
    fn schema() -> String {
        let schema = indoc! {
            r#"model Post {
              #id(id, Int, @id)
              user_id  Int
              user_age Int?
              User     User? @relation(fields: [user_id, user_age], references: [nr, age])

          }

          model User {
            #id(id, Int, @id)
              nr   Int
              age  Int
              Post Post[]

              @@unique([nr, age], name: "user_unique")
          }"#
        };

        schema.to_owned()
    }

    // "A One to Many relation with mixed requiredness" should "be writable and readable"
    #[connector_test(exclude(MySql(5.6), MongoDb))]
    async fn one2m_mix_required_writable_readable(runner: Runner) -> TestResult<()> {
        use query_tests_setup::{ConnectorVersion, VitessVersion::*};

        // Setup user
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation{createOneUser(data:{id: 1, nr:1, age: 1}){id, nr, age, Post{id}}}"#),
          @r###"{"data":{"createOneUser":{"id":1,"nr":1,"age":1,"Post":[]}}}"###
        );

        // Null constraint violation
        assert_error!(
            &runner,
            r#"mutation{createOnePost(data: { id: 1 }) { id, user_id, user_age, User { id } }}"#,
            2011
        );

        //Success
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation{createOnePost(data:{id: 1, user_id:1}){id, user_id, user_age, User{id}}}"#),
          @r###"{"data":{"createOnePost":{"id":1,"user_id":1,"user_age":null,"User":null}}}"###
        );

        // Foreign key violation, which doesn't happen on PlanetScale.
        if !matches!(
            runner.connector_version(),
            ConnectorVersion::Vitess(Some(PlanetscaleJsNapi)) | ConnectorVersion::Vitess(Some(PlanetscaleJsWasm))
        ) {
            assert_error!(
                &runner,
                r#"mutation{createOnePost(data:{id: 2, user_id:2, user_age: 2}){id, user_id, user_age, User{id}}}"#,
                2003
            );
        }

        // Success
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation{createOnePost(data:{id: 2, user_id:1, user_age: 1}){id, user_id, user_age, User{id}}}"#),
          @r###"{"data":{"createOnePost":{"id":2,"user_id":1,"user_age":1,"User":{"id":1}}}}"###
        );

        Ok(())
    }
}
