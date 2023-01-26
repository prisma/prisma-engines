use query_engine_tests::*;

#[test_suite(capabilities(NativeUpsert))]
mod native_upsert {

    #[connector_test(schema(user))]
    async fn should_upsert_on_single_unique(runner: Runner) -> TestResult<()> {
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

        insta::assert_snapshot!(
          run_query!(&runner, upsert),
          @r###"{"data":{"upsertOneUser":{"id":1,"last_name":"world-updated"}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(user))]
    async fn should_upsert_on_id(runner: Runner) -> TestResult<()> {
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

        insta::assert_snapshot!(
          run_query!(&runner, upsert),
          @r###"{"data":{"upsertOneUser":{"id":2,"last_name":"world-updated","first_name":"hello","email":"hello-updated@example.com"}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(user))]
    async fn should_upsert_on_unique_list(runner: Runner) -> TestResult<()> {
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

        insta::assert_snapshot!(
          run_query!(&runner, upsert),
          @r###"{"data":{"upsertOneUser":{"id":1,"last_name":"world","first_name":"hello","email":"hello-updated@example.com"}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(user))]
    async fn should_not_use_native_upsert_on_two_uniques(runner: Runner) -> TestResult<()> {
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

        insta::assert_snapshot!(
          run_query!(&runner, upsert),
          @r###"{"data":{"upsertOneUser":{"id":1,"last_name":"world","first_name":"hello","email":"hello-updated@example.com"}}}"###
        );

        Ok(())
    }

    // Should not use native upsert when the unique field values defined in the where clause
    // do not match the same uniques fields in the create clause
    #[connector_test(schema(user))]
    async fn should_not_use_if_where_and_create_different(runner: Runner) -> TestResult<()> {
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

    #[connector_test(schema(user))]
    async fn should_not_if_missing_update(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            upsertOneUser(
              where: {email: "email1"}
              create: {
                id: 1,
                email: "email1",
                first_name: "first",
                last_name: "last",
              }
              update: {
              }
            ){
              id,
              email
            }
          }"#),
          @r###"{"data":{"upsertOneUser":{"id":1,"email":"email1"}}}"###
        );

        Ok(())
    }

    pub fn relations() -> String {
        let schema = indoc! {
            "
          model User {
            #id(id, Int, @id)
            first_name String
            last_name  String
            email      String    @unique
            birthday   DateTime?
            location Location?
          }

          model Location {
            #id(id, Int, @id)
          
            userId Int @unique
            user User @relation(fields: [userId], references: [id])
          }
        "
        };

        schema.to_owned()
    }

    #[connector_test(schema(relations))]
    async fn should_not_if_has_nested_select(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            upsertOneUser(
              where: {email: "email1"}
              create: {
                id: 1,
                first_name: "first",
                last_name: "last",
                email: "email1",
              }
              update: {
                email: "email1-updated",
              }
            ){
              id,
              email,
              location {
                id
              }
            }
          }"#),
          @r###"{"data":{"upsertOneUser":{"id":1,"email":"email1","location":null}}}"###
        );

        Ok(())
    }

    pub fn compound_id() -> String {
        let schema = indoc! {
            "model TestModel {
              id1 Int
              id2 Int
              field String

              @@id([id1, id2])
          }"
        };

        schema.to_owned()
    }

    #[connector_test(schema(compound_id))]
    async fn should_upsert_on_compound_id(runner: Runner) -> TestResult<()> {
        let upsert = r#"
          mutation {
            upsertOneTestModel(
              where: {id1_id2: {
                id1: 1,
                id2: 2
              }},
              create: {
                id1: 1,
                id2: 2,
                field: "hello",
              },
              update: {
                field: "hello-updated",
              }
            ) {
              id1,
              id2,
              field
            }
          }
        "#;

        insta::assert_snapshot!(
          run_query!(&runner, upsert),
          @r###"{"data":{"upsertOneTestModel":{"id1":1,"id2":2,"field":"hello"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, upsert),
          @r###"{"data":{"upsertOneTestModel":{"id1":1,"id2":2,"field":"hello-updated"}}}"###
        );

        Ok(())
    }
}
