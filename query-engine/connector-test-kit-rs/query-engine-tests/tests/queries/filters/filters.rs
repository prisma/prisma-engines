use query_engine_tests::*;

#[test_suite(schema(schema))]
mod filter_spec {
    use indoc::indoc;
    use query_engine_tests::assert_error;

    fn schema() -> String {
        let schema = indoc! {
            r#"
            model User {
                #id(id, String, @id, @default(cuid()))
                unique     Int      @unique
                name       String?
                optional   String?
                vehicle_id String? @unique

                ride Vehicle? @relation(fields: [vehicle_id], references: [id])
            }

            model Vehicle {
                #id(id, String, @id, @default(cuid()))
                unique Int     @unique
                brand  String?
                parked Boolean?

                owner  User?
            }

            model ParkingLot {
                #id(id, String, @id, @default(cuid()))
                unique   Int    @unique
                area     String?
                size     Float?
                capacity Int?
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn no_filter(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          &user_uniques(&runner, "").await?,
          @r###"{"data":{"findManyUser":[{"unique":1},{"unique":2},{"unique":3},{"unique":4}]}}"###
        );

        insta::assert_snapshot!(
          &vehicle_uniques(&runner, "").await?,
          @r###"{"data":{"findManyVehicle":[{"unique":1},{"unique":2},{"unique":3}]}}"###
        );

        insta::assert_snapshot!(
          &lot_uniques(&runner, "").await?,
          @r###"{"data":{"findManyParkingLot":[{"unique":1},{"unique":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn simple(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          &user_uniques(&runner, r#"(where: { name: { equals: "John" }})"#).await?,
          @r###"{"data":{"findManyUser":[{"unique":4}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn inverted_simple(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          &user_uniques(&runner, r#"(where: { name: { not: { equals: "John" }}})"#).await?,
          @r###"{"data":{"findManyUser":[{"unique":1},{"unique":2},{"unique":3}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn implicit_not_equals(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          &user_uniques(&runner, r#"(where: { name: { not: "John" }})"#).await?,
          @r###"{"data":{"findManyUser":[{"unique":1},{"unique":2},{"unique":3}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn implicit_equals(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          &user_uniques(&runner, r#"(where: { name: "John" })"#).await?,
          @r###"{"data":{"findManyUser":[{"unique":4}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn implicit_equals_null(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          &user_uniques(&runner, r#"(where: { name: null })"#).await?,
          @r###"{"data":{"findManyUser":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn in_null(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        match_connector_result!(
          &runner,
          user_uniques_query(r#"(where: { optional: { in: null }})"#),
          // MongoDB excludes undefined fields
          MongoDb(_) => vec![r#"{"data":{"findManyUser":[]}}"#],
          _ => vec![r#"{"data":{"findManyUser":[{"unique":1},{"unique":2},{"unique":3},{"unique":4}]}}"#]
        );

        Ok(())
    }

    #[connector_test]
    async fn in_list(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          &user_uniques(&runner, r#"(where: { name: { in: ["Bernd", "Paul"] }})"#).await?,
          @r###"{"data":{"findManyUser":[{"unique":1},{"unique":2}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn not_in_list(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          &user_uniques(&runner, r#"(where: { name: { notIn: ["Bernd", "Paul"] }})"#).await?,
          @r###"{"data":{"findManyUser":[{"unique":3},{"unique":4}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn not_in_null(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          &user_uniques(&runner, r#"(where: { name: { notIn: null }})"#).await?,
          @r###"{"data":{"findManyUser":[{"unique":1},{"unique":2},{"unique":3},{"unique":4}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn relation_null(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          &user_uniques(&runner, r#"(where: { ride: { is: null }})"#).await?,
          @r###"{"data":{"findManyUser":[{"unique":4}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn and(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          &user_uniques(&runner, r#"(where: { AND: [ { unique: { gt: 2 }},{ name: { startsWith: "P" }}]})"#).await?,
          @r###"{"data":{"findManyUser":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn empty_and(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          &user_uniques(&runner, r#"(where: { AND: []})"#).await?,
          @r###"{"data":{"findManyUser":[{"unique":1},{"unique":2},{"unique":3},{"unique":4}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn or(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          &user_uniques(&runner, r#"(where: { OR: [{ unique: { gt: 2 }}, { name: { startsWith: "P" }}]})"#).await?,
          @r###"{"data":{"findManyUser":[{"unique":1},{"unique":3},{"unique":4}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn empty_or(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          &user_uniques(&runner, r#"(where: { OR: [] })"#).await?,
          @r###"{"data":{"findManyUser":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn empty_not(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          &user_uniques(&runner, r#"(where: { NOT: [] })"#).await?,
          @r###"{"data":{"findManyUser":[{"unique":1},{"unique":2},{"unique":3},{"unique":4}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn not(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          &user_uniques(&runner, r#"(where: { NOT: { name: { startsWith: "P" }}})"#).await?,
          @r###"{"data":{"findManyUser":[{"unique":2},{"unique":3},{"unique":4}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn not_not(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          &user_uniques(&runner, r#"(where: { NOT: { NOT: { name: { startsWith: "P" }} }})"#).await?,
          @r###"{"data":{"findManyUser":[{"unique":1}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn not_list(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          &user_uniques(&runner, r#"(where: { NOT: [{ name: { contains: "e" } }, { unique: { equals: 1 } }]})"#).await?,
          @r###"{"data":{"findManyUser":[{"unique":4}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn nested_filter(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          &user_uniques(&runner, r#"(where: { ride: { is: { brand: { startsWith: "P" }}}})"#).await?,
          @r###"{"data":{"findManyUser":[{"unique":1}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn starts_with(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          &user_uniques(&runner, r#"(where: { name: { startsWith: "P"}})"#).await?,
          @r###"{"data":{"findManyUser":[{"unique":1}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn contains(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          &user_uniques(&runner, r#"(where: { name: { contains: "n" }})"#).await?,
          @r###"{"data":{"findManyUser":[{"unique":2},{"unique":4}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn greater_than(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          &lot_uniques(&runner, r#"(where: {size: { gt: 100.500000000001 }})"#).await?,
          @r###"{"data":{"findManyParkingLot":[{"unique":1}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn inverted_null(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          &user_uniques(&runner, r#"(where: { name: { not: null }})"#).await?,
          @r###"{"data":{"findManyUser":[{"unique":1},{"unique":2},{"unique":3},{"unique":4}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn inverted_null_required(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        assert_error!(
            runner,
            "{ findManyUser(where: { unique: { not: null }}){ unique } }",
            2009,
            "A value is required but not set"
        );

        Ok(())
    }

    fn user_uniques_query(filter: &str) -> String {
        format!(r#"query {{ findManyUser{filter} {{ unique }} }}"#)
    }

    async fn user_uniques(runner: &Runner, filter: &str) -> TestResult<String> {
        let result = runner.query(user_uniques_query(filter)).await?;

        result.assert_success();
        Ok(result.to_string())
    }

    async fn vehicle_uniques(runner: &Runner, filter: &str) -> TestResult<String> {
        let result = runner
            .query(format!(r#"query {{ findManyVehicle{filter} {{ unique }} }}"#))
            .await?;

        result.assert_success();
        Ok(result.to_string())
    }

    async fn lot_uniques(runner: &Runner, filter: &str) -> TestResult<String> {
        let result = runner
            .query(format!(r#"query {{ findManyParkingLot{filter} {{ unique }} }}"#))
            .await?;

        result.assert_success();
        Ok(result.to_string())
    }

    async fn test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(r#"mutation { createOneUser(data: { name: "Paul", unique: 1, ride: { create: { brand: "Porsche", unique: 1, parked: true }}}) { id }}"#)
            .await?.assert_success();

        runner
            .query(r#"mutation { createOneUser(data: { name: "Bernd", unique: 2, ride: { create: { brand: "BMW", unique: 2, parked: false }}}) { id }}"#)
            .await?.assert_success();

        runner
            .query(r#"mutation { createOneUser(data: { name: "Michael", unique: 3, ride: { create: { brand: "Mercedes", unique: 3, parked: true }}}) { id }}"#)
            .await?.assert_success();

        runner
            .query(r#"mutation { createOneUser(data: { name: "John", unique: 4 }) { id }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneParkingLot(data: { area: "PrenzlBerg", unique: 1, capacity: 12, size: 300.5 }) { id }}"#)
            .await?.assert_success();

        runner
            .query(r#"mutation { createOneParkingLot(data: { area: "Moabit", unique: 2, capacity: 34, size: 100.5 }) { id }}"#)
            .await?.assert_success();

        Ok(())
    }
}
