use query_engine_tests::*;

// https://stackoverflow.com/questions/4380813/how-to-get-rid-of-mysql-error-prepared-statement-needs-to-be-re-prepared
// Looks like there's a bug with create view stmt on MariaDB.
// On D1, the migration setup fails because Schema Engine doesn't know anything about Driver Adapters.
#[test_suite(schema(schema), exclude(MongoDb, MySQL("mariadb"), Vitess, Sqlite("cfd1")))]
mod views {
    use query_engine_tests::{Runner, connector_test, run_query};

    fn schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              firstName       String
              lastName String
            }

            view TestView {
              #id(id, Int, @id)

              firstName String
              lastName  String
              fullName  String
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn simple_read(runner: Runner) -> TestResult<()> {
        create_test_data(&runner, "simple_read").await?;

        // find many
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestView { id firstName, lastName, fullName } }"#),
          @r###"{"data":{"findManyTestView":[{"id":1,"firstName":"John","lastName":"Doe","fullName":"John Doe"},{"id":2,"firstName":"Jane","lastName":"Doe","fullName":"Jane Doe"},{"id":3,"firstName":"Bob","lastName":"Maurane","fullName":"Bob Maurane"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn filtered_read(runner: Runner) -> TestResult<()> {
        create_test_data(&runner, "filtered_read").await?;

        // Filter on column
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findUniqueTestView(where: { id: 1 }) { id fullName } }"#),
          @r###"{"data":{"findUniqueTestView":{"id":1,"fullName":"John Doe"}}}"###
        );

        // Filter on computed column of the view
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestView(where: { fullName: "Jane Doe" }) { id fullName } }"#),
          @r###"{"data":{"findManyTestView":[{"id":2,"fullName":"Jane Doe"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn sorted_read(runner: Runner) -> TestResult<()> {
        create_test_data(&runner, "sorted_read").await?;

        // Order by on computed column of the view
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestView(orderBy: { fullName: asc }) { id fullName } }"#),
          @r###"{"data":{"findManyTestView":[{"id":3,"fullName":"Bob Maurane"},{"id":2,"fullName":"Jane Doe"},{"id":1,"fullName":"John Doe"}]}}"###
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner, schema_name: &str) -> TestResult<()> {
        migrate_view(runner, schema_name).await?;

        create_test_model(runner, r#"{ id: 1, firstName: "John", lastName: "Doe" }"#).await?;
        create_test_model(runner, r#"{ id: 2, firstName: "Jane", lastName: "Doe" }"#).await?;
        create_test_model(runner, r#"{ id: 3, firstName: "Bob", lastName: "Maurane" }"#).await?;

        Ok(())
    }

    async fn migrate_view(runner: &Runner, schema_name: &str) -> TestResult<()> {
        let sql = migrate_view_sql(runner, schema_name).await;

        runner.raw_execute(sql).await?;

        Ok(())
    }

    // schema name must be the name of the test in which it's called.
    async fn migrate_view_sql(runner: &Runner, schema_name: &str) -> String {
        match runner.connector_version() {
            ConnectorVersion::Postgres(_)
            | ConnectorVersion::CockroachDb(_)
             => {
                r#"CREATE VIEW "TestView" AS SELECT "TestModel".id, "TestModel"."firstName", "TestModel"."lastName", CONCAT("TestModel"."firstName", ' ', "TestModel"."lastName") as "fullName" From "TestModel""#.to_owned()
            }
            ConnectorVersion::MySql(_) | ConnectorVersion::Vitess(_)
             => {
              r#"CREATE VIEW TestView AS SELECT TestModel.*, CONCAT(TestModel.firstName, ' ', TestModel.lastName) AS "fullName" FROM TestModel"#.to_owned()
            },
            ConnectorVersion::Sqlite(_) => {
              r#"CREATE VIEW TestView AS SELECT TestModel.*, TestModel.firstName || ' ' || TestModel.lastName AS "fullName" FROM TestModel"#.to_owned()
            }
            ConnectorVersion::SqlServer(_) => {
              format!(r#"CREATE VIEW [views_{schema_name}].[TestView] AS SELECT [views_{schema_name}].[TestModel].[id], [views_{schema_name}].[TestModel].[firstName], [views_{schema_name}].[TestModel].[lastName], CONCAT([views_{schema_name}].[TestModel].[firstName], ' ', [views_{schema_name}].[TestModel].[lastName]) as "fullName" FROM [views_{schema_name}].[TestModel];"#)
            },
            ConnectorVersion::MongoDb(_) => unreachable!(),
        }
    }

    async fn create_test_model(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTestModel(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}
