use query_engine_tests::*;

#[test_suite(capabilities(NativeUpsert))]
mod native_upsert {

    #[connector_test(schema(user))]
    async fn should_upsert_on_single_unique(mut runner: Runner) -> TestResult<()> {
        let upsert = r#"
          mutation {
            upsertOneUser(
              where: {email: "hello@example.com"},
              create: {
                id: 1,
                email: "hello@example.com",
                first_name: "hello",
                last_name: "world"
              },
              update: {
                last_name: "world-updated"
              }
            ) {
              id,
              last_name
            }
          }
        "#;

        insta::assert_snapshot!(
          run_query!(&runner, upsert),
          @r###"{"data":{"upsertOneUser":{"id":1,"last_name":"world"}}}"###
        );

        assert_used_native_upsert(&mut runner).await;

        insta::assert_snapshot!(
          run_query!(&runner, upsert),
          @r###"{"data":{"upsertOneUser":{"id":1,"last_name":"world-updated"}}}"###
        );

        assert_used_native_upsert(&mut runner).await;

        Ok(())
    }

    #[connector_test(schema(user))]
    async fn should_upsert_on_id(mut runner: Runner) -> TestResult<()> {
        let upsert = r#"
          mutation {
            upsertOneUser(
              where: {id: 1},
              create: {
                id: 1,
                email: "hello@example.com",
                first_name: "hello",
                last_name: "world"
              },
              update: {
                last_name: "world-updated",
                email: "hello-updated@example.com",
                id: 2
              }
            ) {
              id,
              last_name,
              first_name,
              email
            }
          }
        "#;

        insta::assert_snapshot!(
          run_query!(&runner, upsert),
          @r###"{"data":{"upsertOneUser":{"id":1,"last_name":"world","first_name":"hello","email":"hello@example.com"}}}"###
        );

        assert_used_native_upsert(&mut runner).await;

        insta::assert_snapshot!(
          run_query!(&runner, upsert),
          @r###"{"data":{"upsertOneUser":{"id":2,"last_name":"world-updated","first_name":"hello","email":"hello-updated@example.com"}}}"###
        );

        assert_used_native_upsert(&mut runner).await;

        Ok(())
    }

    #[connector_test(schema(user))]
    async fn should_upsert_on_unique_list(mut runner: Runner) -> TestResult<()> {
        let upsert = r#"
          mutation {
            upsertOneUser(
              where: {first_name_last_name: {
                first_name: "hello",
                last_name: "world"
              }},
              create: {
                id: 1,
                email: "hello@example.com",
                first_name: "hello",
                last_name: "world"
              },
              update: {
                email: "hello-updated@example.com",
              }
            ) {
              id,
              last_name,
              first_name,
              email
            }
          }
        "#;

        insta::assert_snapshot!(
          run_query!(&runner, upsert),
          @r###"{"data":{"upsertOneUser":{"id":1,"last_name":"world","first_name":"hello","email":"hello@example.com"}}}"###
        );

        assert_used_native_upsert(&mut runner).await;

        insta::assert_snapshot!(
          run_query!(&runner, upsert),
          @r###"{"data":{"upsertOneUser":{"id":1,"last_name":"world","first_name":"hello","email":"hello-updated@example.com"}}}"###
        );

        assert_used_native_upsert(&mut runner).await;

        Ok(())
    }

    #[connector_test(schema(user))]
    async fn should_not_use_native_upsert_on_two_uniques(mut runner: Runner) -> TestResult<()> {
        let upsert = r#"
          mutation {
            upsertOneUser(
              where: {
                id: 1,
                email: "hello@example.com",
              },
              create: {
                id: 1,
                email: "hello@example.com",
                first_name: "hello",
                last_name: "world"
              },
              update: {
                email: "hello-updated@example.com",
              }
            ) {
              id,
              last_name,
              first_name,
              email
            }
          }
        "#;

        insta::assert_snapshot!(
          run_query!(&runner, upsert),
          @r###"{"data":{"upsertOneUser":{"id":1,"last_name":"world","first_name":"hello","email":"hello@example.com"}}}"###
        );

        assert_not_used_native_upsert(&mut runner).await;

        insta::assert_snapshot!(
          run_query!(&runner, upsert),
          @r###"{"data":{"upsertOneUser":{"id":1,"last_name":"world","first_name":"hello","email":"hello-updated@example.com"}}}"###
        );

        assert_not_used_native_upsert(&mut runner).await;

        Ok(())
    }

    // Should not use native upsert when the unique field values defined in the where clause
    // do not match the same uniques fields in the create clause
    #[connector_test(schema(user))]
    async fn should_not_use_if_where_and_create_different(mut runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
                createOneUser(
                  data: {
                    id: 1,
                    first_name: "first",
                    last_name: "last",
                    email: "email1"
                  }
                ) {
                  id
                }
            }"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            upsertOneUser(
              where: {email: "email1"}
              create: {
                id: 1,
                email: "another-email",
                first_name: "first",
                last_name: "last",
              }
              update: {
                email: { set:"email-updated" }
              }
            ){
              id,
              email
            }
          }"#),
          @r###"{"data":{"upsertOneUser":{"id":1,"email":"email-updated"}}}"###
        );

        assert_not_used_native_upsert(&mut runner).await;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findUniqueUser(where: {id: 1}){
              email
            }
          }"#),
          @r###"{"data":{"findUniqueUser":{"email":"email-updated"}}}"###
        );

        Ok(())
    }

    async fn assert_used_native_upsert(runner: &mut Runner) {
        let logs = runner.get_logs().await;
        let did_upsert = logs.iter().any(|l| l.contains("ON CONFLICT"));
        assert!(did_upsert);
    }

    async fn assert_not_used_native_upsert(runner: &mut Runner) {
        let logs = runner.get_logs().await;
        let did_upsert = logs.iter().any(|l| l.contains("ON CONFLICT"));
        assert!(!did_upsert);
    }
}
