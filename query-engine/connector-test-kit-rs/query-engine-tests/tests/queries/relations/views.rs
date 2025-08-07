use query_engine_tests::*;

// https://stackoverflow.com/questions/4380813/how-to-get-rid-of-mysql-error-prepared-statement-needs-to-be-re-prepared
// Looks like there's a bug with create view stmt on MariaDB.
// On D1, the migration setup fails because Schema Engine doesn't know anything about Driver Adapters.
#[test_suite(schema(schema), exclude(MongoDb, MySQL("mariadb"), Vitess, Sqlite("cfd1")))]
mod views_with_relations {
    use query_engine_tests::{Runner, connector_test, run_query};

    fn schema() -> String {
        let schema = indoc! {
            r#"
                model TestModel {
                  #id(id, Int, @id)
                  firstName       String
                  lastName String
                }

                model Child {
                  #id(id, Int, @id)
                  name String
                  viewId Int?
                  view TestView? @relation(fields: [viewId], references: [id])
                }

                view TestView {
                  #id(id, Int, @unique)
                  firstName String
                  lastName  String
                  fullName  String
                  children Child[]
                }
            "#
        };
        schema.to_owned()
    }

    #[connector_test]
    async fn nested_read(runner: Runner) -> TestResult<()> {
        create_test_data(&runner, "nested_read").await?;

        // find many with nested read
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestView { id, firstName, lastName, fullName children { id, name } } }"#),
          @r###"{"data":{"findManyTestView":[{"id":1,"firstName":"John","lastName":"Doe","fullName":"John Doe","children":[{"id":1,"name":"Derek"},{"id":2,"name":"Kevin"}]},{"id":2,"firstName":"Jane","lastName":"Doe","fullName":"Jane Doe","children":[]},{"id":3,"firstName":"Bob","lastName":"Maurane","fullName":"Bob Maurane","children":[]}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn filtered_read(runner: Runner) -> TestResult<()> {
        create_test_data(&runner, "filtered_read").await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestView(where: { children: { some: { name: "Derek" } } }) { id fullName children { name } } }"#),
          @r###"{"data":{"findManyTestView":[{"id":1,"fullName":"John Doe","children":[{"name":"Derek"},{"name":"Kevin"}]}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn sorted_read(runner: Runner) -> TestResult<()> {
        create_test_data(&runner, "sorted_read").await?;

        is_one_of!(
            run_query!(
                &runner,
                r#"{ findManyTestView(orderBy: { children: { _count: asc } }) { id _count { children } } }"#
            ),
            [
                r#"{"data":{"findManyTestView":[{"id":2,"_count":{"children":0}},{"id":3,"_count":{"children":0}},{"id":1,"_count":{"children":2}}]}}"#,
                r#"{"data":{"findManyTestView":[{"id":3,"_count":{"children":0}},{"id":2,"_count":{"children":0}},{"id":1,"_count":{"children":2}}]}}"#
            ]
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner, schema_name: &str) -> TestResult<()> {
        migrate_view(runner, schema_name).await?;
        create_test_model(runner, r#"{ id: 1, firstName: "John", lastName: "Doe" }"#).await?;
        create_test_model(runner, r#"{ id: 2, firstName: "Jane", lastName: "Doe" }"#).await?;
        create_test_model(runner, r#"{ id: 3, firstName: "Bob", lastName: "Maurane" }"#).await?;

        create_child(runner, r#"{ id: 1, name: "Derek" viewId: 1 }"#).await?;
        create_child(runner, r#"{ id: 2, name: "Kevin" viewId: 1 }"#).await?;

        Ok(())
    }

    async fn migrate_view(runner: &Runner, schema_name: &str) -> TestResult<()> {
        let sql = migrate_view_sql(runner, schema_name).await;
        runner.raw_execute(sql).await
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
                  format!(r#"CREATE VIEW [views_with_relations_{schema_name}].[TestView] AS SELECT [views_with_relations_{schema_name}].[TestModel].[id], [views_with_relations_{schema_name}].[TestModel].[firstName], [views_with_relations_{schema_name}].[TestModel].[lastName], CONCAT([views_with_relations_{schema_name}].[TestModel].[firstName], ' ', [views_with_relations_{schema_name}].[TestModel].[lastName]) as "fullName" FROM [views_with_relations_{schema_name}].[TestModel];"#)
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

    async fn create_child(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneChild(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}
