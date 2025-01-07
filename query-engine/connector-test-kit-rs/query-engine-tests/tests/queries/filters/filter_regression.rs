//! Regression tests for nullable fields and 1:m filters.
use query_engine_tests::*;

/// Basic filter regression 1:m relation tests.
#[test_suite(schema(schema))]
mod fr_one_to_m {
    use indoc::indoc;
    use query_engine_tests::run_query;

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
    async fn work_with_nulls(runner: Runner) -> TestResult<()> {
        test_location(&runner, r#"{ id: 310, name: "A" }"#).await?;
        test_location(&runner, r#"{ id: 311, name: "A" }"#).await?;
        test_location(&runner, r#"{ id: 314, name: "A" }"#).await?;
        test_location(&runner, r#"{ id: 312, name: "B" }"#).await?;
        test_location(&runner, r#"{ id: 317, name: "B" }"#).await?;
        test_location(&runner, r#"{ id: 313, name: "C" }"#).await?;
        test_location(&runner, r#"{ id: 315, name: "C" }"#).await?;
        test_location(&runner, r#"{ id: 316, name: "D" }"#).await?;

        test_company(
            &runner,
            r#"{ id: 134, name: "1", locations: { connect: [{ id: 310 }, { id: 312 }, { id: 313 }] }}"#,
        )
        .await?;

        test_company(
            &runner,
            r#"{ id: 135, name: "2", locations: { connect: [{ id: 311 }, { id: 314 }] }}"#,
        )
        .await?;

        test_company(
            &runner,
            r#"{ id: 136, name: "3", locations: { connect: [{ id: 315 }, { id: 317 }] }}"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyCompany(where: { locations: { none: { name: { equals: "D" }}}}){ id }}"#),
          @r###"{"data":{"findManyCompany":[{"id":134},{"id":135},{"id":136}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyCompany(where: { locations: { every: { name: { equals: "A" }}}}){ id }}"#),
          @r###"{"data":{"findManyCompany":[{"id":135}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyLocation(where: { company: { is: { id: { equals: 135 }}}}){ id }}"#),
          @r###"{"data":{"findManyLocation":[{"id":311},{"id":314}]}}"###
        );

        Ok(())
    }
}

/// Filter regression 1:m relation tests with compound ids.
#[test_suite(schema(schema), capabilities(CompoundIds))]
mod fr_compound_one_to_m {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        use indoc::indoc;

        let schema = indoc! { "
            model Location {
                #id(id, Int, @id)
                name       String?
                companyId  Int?
                companyId2 Int?
                company    Company?  @relation(fields: [companyId, companyId2], references: [id, id2])
            }

            model Company {
                id        Int
                id2       Int
                name      String?
                locations Location[]

                @@id([id, id2])
            }
        "};

        schema.to_owned()
    }

    #[connector_test]
    async fn work_with_nulls(runner: Runner) -> TestResult<()> {
        test_location(&runner, r#"{ id: 310, name: "A"}"#).await?;
        test_location(&runner, r#"{ id: 311, name: "A"}"#).await?;
        test_location(&runner, r#"{ id: 314, name: "A"}"#).await?;
        test_location(&runner, r#"{ id: 312, name: "B"}"#).await?;
        test_location(&runner, r#"{ id: 317, name: "B"}"#).await?;
        test_location(&runner, r#"{ id: 313, name: "C"}"#).await?;
        test_location(&runner, r#"{ id: 315, name: "C"}"#).await?;
        test_location(&runner, r#"{ id: 316, name: "D"}"#).await?;

        test_company(
            &runner,
            r#"{ id: 134, id2: 134, name: "1", locations: { connect: [{ id: 310 }, { id: 312 }, { id: 313 }]}}"#,
        )
        .await?;

        test_company(
            &runner,
            r#"{ id: 135, id2: 135, name: "2", locations: { connect: [{ id: 311 }, { id: 314 }]}}"#,
        )
        .await?;

        test_company(
            &runner,
            r#"{ id: 136, id2: 136, name: "3", locations: { connect: [{ id: 315 }, { id: 317 }]}}"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyCompany(where: { locations: { none: { name: { equals: "D" }}}}){ id }}"#),
          @r###"{"data":{"findManyCompany":[{"id":134},{"id":135},{"id":136}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyCompany(where: { locations: { every: { name: { equals: "A" }}}}){ id }}"#),
          @r###"{"data":{"findManyCompany":[{"id":135}]}}"###
        );

        Ok(())
    }
}

/// Filter regression m:n relation tests.
#[test_suite(schema(schema))]
mod fr_m_to_n {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! { "
            model Location {
                #id(id, Int, @id)
                name String?
                #m2m(companies, Company[], id, Int)
            }

            model Company {
                #id(id, Int, @id)
                name String?
                #m2m(locations, Location[], id, Int)
            }
        "};

        schema.to_owned()
    }

    #[connector_test]
    async fn work_with_nulls(runner: Runner) -> TestResult<()> {
        test_location(&runner, r#"{ id: 311, name: "A"}"#).await?;
        test_location(&runner, r#"{ id: 310, name: "A"}"#).await?;
        test_location(&runner, r#"{ id: 314, name: "A"}"#).await?;
        test_location(&runner, r#"{ id: 312, name: "B"}"#).await?;
        test_location(&runner, r#"{ id: 317, name: "B"}"#).await?;
        test_location(&runner, r#"{ id: 313, name: "C"}"#).await?;
        test_location(&runner, r#"{ id: 315, name: "C"}"#).await?;
        test_location(&runner, r#"{ id: 316, name: "D"}"#).await?;

        test_company(
            &runner,
            r#"{ id: 134, name: "1", locations: { connect: [{ id: 310 }, { id: 312 }, { id: 313 }]}}"#,
        )
        .await?;

        test_company(
            &runner,
            r#"{ id: 135, name: "2", locations: { connect: [{ id: 311 }, { id: 314 }]}}"#,
        )
        .await?;

        test_company(
            &runner,
            r#"{ id: 136, name: "3", locations: { connect: [{ id: 315 }, { id: 317 }]}}"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"
              query {
                findManyCompany(
                  where: { locations: { none: { name: { equals: "D" }}}}
                  orderBy: { id: asc }
                ) { id }
              }"#),
          @r###"{"data":{"findManyCompany":[{"id":134},{"id":135},{"id":136}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyCompany(where: { locations: { every: { name: { equals: "A" }}}}){ id }}"#),
          @r###"{"data":{"findManyCompany":[{"id":135}]}}"###
        );

        Ok(())
    }
}

async fn test_location(runner: &Runner, data: &str) -> TestResult<()> {
    runner
        .query(format!("mutation {{ createOneLocation(data: {data}) {{ id }} }}"))
        .await?
        .assert_success();

    Ok(())
}

async fn test_company(runner: &Runner, data: &str) -> TestResult<()> {
    runner
        .query(format!("mutation {{ createOneCompany(data: {data}) {{ id }} }}"))
        .await?
        .assert_success();

    Ok(())
}
