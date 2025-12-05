use query_engine_tests::*;

#[test_suite]
mod default_value {
    use indoc::indoc;
    use query_engine_tests::{run_query, run_query_json};

    fn schema_all_default() -> String {
        let schema = indoc! {
            r#"model Service {
              #id(id, String, @id, @default(cuid()))
            }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_all_default))]
    async fn default_field_omitted_in_data(runner: Runner) -> TestResult<()> {
        runner
            .query(r#"mutation { createOneService(data: {}) { id } }"#)
            .await?
            .assert_success();

        Ok(())
    }

    #[connector_test(schema(schema_all_default))]
    async fn default_field_omitted_without_data(runner: Runner) -> TestResult<()> {
        runner
            .query(r#"mutation { createOneService { id } }"#)
            .await?
            .assert_success();

        Ok(())
    }

    fn schema_string() -> String {
        let schema = indoc! {
            r#"model ScalarModel {
              #id(id, Int, @id)
              reqString String? @default(value: "default")
            }"#
        };

        schema.to_owned()
    }

    // "A Create Mutation on a non-list field" should "utilize the defaultValue"
    #[connector_test(schema(schema_string))]
    async fn non_list_field(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneScalarModel(data: { id: 1 }){ reqString } }"#),
          @r###"{"data":{"createOneScalarModel":{"reqString":"default"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyScalarModel{reqString}}"#),
          @r###"{"data":{"findManyScalarModel":[{"reqString":"default"}]}}"###
        );

        Ok(())
    }

    fn schema_int() -> String {
        let schema = indoc! {
            r#"model Service {
              #id(id, Int, @id)
              name String
              int  Int?   @default(value: 1)
            }"#
        };

        schema.to_owned()
    }

    // "The default value" should "work for int"
    #[connector_test(schema(schema_int))]
    async fn int_field(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneService(
              data:{
                id: 1,
                name: "issue1820"
              }
            ){
              name
              int
            }
          }"#),
          @r###"{"data":{"createOneService":{"name":"issue1820","int":1}}}"###
        );

        Ok(())
    }

    fn schema_enum() -> String {
        let schema = indoc! {
            r#"enum IsActive{
              Yes
              No
            }

            model Service {
              #id(id, Int, @id)
              name         String
              description  String?
              unit         String?
              active       IsActive? @default(value: Yes)
            }"#
        };

        schema.to_owned()
    }

    // "The default value" should "work for enums"
    // TODO: Flaky test on Cockroach, re-enable once figured out
    #[connector_test(schema(schema_enum), exclude(Sqlite, SqlServer, CockroachDb))]
    async fn enum_field(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneService(
              data:{
                id: 1,
                name: "issue1820"
              }
            ){
              name
              active
            }
          }"#),
          @r###"{"data":{"createOneService":{"name":"issue1820","active":"Yes"}}}"###
        );

        Ok(())
    }

    fn schema_datetime() -> String {
        let schema = indoc! {
            r#"model User {
              #id(id, Int, @id)
              name      String
              createdAt DateTime @default(now())
              updatedAt DateTime @updatedAt
            }"#
        };

        schema.to_owned()
    }

    // "The default value for updatedAt and createdAt" should "not be set if specific values are passed on create"
    #[connector_test(schema(schema_datetime))]
    async fn updated_at_created_at(runner: Runner) -> TestResult<()> {
        let res = run_query_json!(
            &runner,
            r#"mutation {
                createOneUser(
                  data:{
                    id: 1,
                    name: "Just Bob"
                    createdAt: "2000-01-01T00:00:00Z"
                    updatedAt: "2001-01-01T00:00:00Z"
                  }
                ){
                  createdAt
                  updatedAt
                }
            }"#
        );

        // We currently have a datetime precision of 3, so Prisma will add .000
        insta::assert_snapshot!(
          &res["data"]["createOneUser"]["createdAt"].to_string(),
          @r###""2000-01-01T00:00:00.000Z""###
        );

        insta::assert_snapshot!(
          &res["data"]["createOneUser"]["updatedAt"].to_string(),
          @r###""2001-01-01T00:00:00.000Z""###
        );

        Ok(())
    }

    fn schema_remapped_enum() -> String {
        let schema = indoc! {
            r#"model User {
              #id(id, Int, @id)
              name      Names    @default(Spiderman) @unique
              age       Int
                    }

            enum Names {
               Spiderman @map("Peter Parker")
               Superman  @map("Clark Kent")
            }"#
        };

        schema.to_owned()
    }

    // "Remapped enum default values" should "work"
    // TODO: Flaky test on Cockroach, re-enable once figured out
    #[connector_test(schema(schema_remapped_enum), exclude(Sqlite, SqlServer, CockroachDb))]
    async fn remapped_enum_field(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneUser(
              data:{
                id: 1,
                age: 21
              }
            ){
              name
            }
          }"#),
          @r###"{"data":{"createOneUser":{"name":"Spiderman"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneUser(
              data:{
                id: 2
                name: Superman
                age: 32
              }
            ){
              name
            }
          }"#),
          @r###"{"data":{"createOneUser":{"name":"Superman"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query {
            findUniqueUser(where:{ name: Superman }) {
              name,
              age
            }
          }"#),
          @r###"{"data":{"findUniqueUser":{"name":"Superman","age":32}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query {
            findManyUser(
              where:{
                name: { in: [Spiderman, Superman] }
              }
              orderBy: { age: asc }
            ){
              name,
              age
            }
          }"#),
          @r###"{"data":{"findManyUser":[{"name":"Spiderman","age":21},{"name":"Superman","age":32}]}}"###
        );

        Ok(())
    }
}
