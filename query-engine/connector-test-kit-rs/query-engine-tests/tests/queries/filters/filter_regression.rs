// "Querying 1:M with relation filters" should "work in the presence of nulls" in {
//     val project = ProjectDsl.fromString {
//       s"""
//          |model Location {
//          |  id        Int     @id
//          |  name      String?
//          |  companyId Int?
//          |  company   Company?  @relation(fields: [companyId], references: [id])
//          |}
//          |
//          |model Company {
//          |  id        Int     @id
//          |  name      String?
//          |  locations Location[]
//          |}
//        """.stripMargin
//     }
//     database.setup(project)

//     server.query("""mutation {createLocation(data: { id: 310, name: "A"}){id}}""", project)
//     server.query("""mutation {createLocation(data: { id: 311, name: "A"}){id}}""", project)
//     server.query("""mutation {createLocation(data: { id: 314, name: "A"}){id}}""", project)
//     server.query("""mutation {createLocation(data: { id: 312, name: "B"}){id}}""", project)
//     server.query("""mutation {createLocation(data: { id: 317, name: "B"}){id}}""", project)
//     server.query("""mutation {createLocation(data: { id: 313, name: "C"}){id}}""", project)
//     server.query("""mutation {createLocation(data: { id: 315, name: "C"}){id}}""", project)
//     server.query("""mutation {createLocation(data: { id: 316, name: "D"}){id}}""", project)

//
//   }

use query_engine_tests::*;

#[test_suite(schema(schema))]
mod one_to_m {
    fn schema() -> String {
        let schema = indoc! { "
            model Location {
              #id(id, Int, @id)
              name      String?
              companyId Int?
              company   Company?  @relation(fields: [companyId], references: [id])
            }

            model Company {
              #id(id, Int, @id)
              name      String?
              locations Location[]
            }
        "};

        schema.to_owned()
    }

    #[connector_test]
    async fn work_with_nulls(runner: &Runner) -> TestResult<()> {
        test_location(runner, r#"{ id: 310, name: "A" }"#).await?;
        test_location(runner, r#"{ id: 311, name: "A" }"#).await?;
        test_location(runner, r#"{ id: 314, name: "A" }"#).await?;
        test_location(runner, r#"{ id: 312, name: "B" }"#).await?;
        test_location(runner, r#"{ id: 317, name: "B" }"#).await?;
        test_location(runner, r#"{ id: 313, name: "C" }"#).await?;
        test_location(runner, r#"{ id: 315, name: "C" }"#).await?;
        test_location(runner, r#"{ id: 316, name: "D" }"#).await?;

        test_company(
            runner,
            r#"{ id: 134, name: "1", locations: { connect: [{ id: 310 }, { id: 312 }, { id: 313 }] }}"#,
        )
        .await?;

        test_company(
            runner,
            r#"{ id: 135, name: "2", locations: { connect: [{ id: 311 }, { id: 314 }] }}"#,
        )
        .await?;

        test_company(
            runner,
            r#"{ id: 136, name: "3", locations: { connect: [{ id: 315 }, { id: 317 }] }}"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyCompany(where: { locations: { none: { name: { equals: "D" }}}}){ id }}"#),
          @r###"{"data":{"findManyCompany":[{"id":134},{"id":135},{"id":136}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyCompany(where: { locations: { every: { name: { equals: "A" }}}}){ id }}"#),
          @r###"{"data":{"findManyCompany":[{"id":135}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyLocation(where: { company: { is: { id: { equals: 135 }}}}){ id }}"#),
          @r###"{"data":{"findManyLocation":[{"id":311},{"id":314}]}}"###
        );

        Ok(())
    }

    async fn test_location(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneLocation(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();

        Ok(())
    }

    async fn test_company(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneCompany(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();

        Ok(())
    }
}
