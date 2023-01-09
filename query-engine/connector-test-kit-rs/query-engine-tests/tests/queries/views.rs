use query_engine_tests::*;

#[test_suite(schema(schema))]
mod views {
    use query_engine_tests::{connector_test, run_query, Runner};

    fn schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
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
              #id(id, Int, @id)

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
    async fn simple_read(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // find many
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestView { id firstName, lastName, fullName } }"#),
          @r###"{"data":{"findManyTestView":[{"id":1,"firstName":"John","lastName":"Doe","fullName":"John Doe"},{"id":2,"firstName":"Jane","lastName":"Doe","fullName":"Jane Doe"},{"id":3,"firstName":"Bob","lastName":"Maurane","fullName":"Bob Maurane"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn nested_read(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // find many with nested read
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestView { id, firstName, lastName, fullName children { id, name } } }"#),
          @r###"{"data":{"findManyTestView":[{"id":1,"firstName":"John","lastName":"Doe","fullName":"John Doe","children":[{"id":1,"name":"Derek"},{"id":2,"name":"Kevin"}]},{"id":2,"firstName":"Jane","lastName":"Doe","fullName":"Jane Doe","children":[]},{"id":3,"firstName":"Bob","lastName":"Maurane","fullName":"Bob Maurane","children":[]}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn filtered_read(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

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

        // Filter on one2many relation
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestView(where: { children: { some: { name: "Derek" } } }) { id fullName children { name } } }"#),
          @r###"{"data":{"findManyTestView":[{"id":1,"fullName":"John Doe","children":[{"name":"Derek"},{"name":"Kevin"}]}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn sorted_read(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        // Order by on computed column of the view
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestView(orderBy: { fullName: asc }) { id fullName } }"#),
          @r###"{"data":{"findManyTestView":[{"id":3,"fullName":"Bob Maurane"},{"id":2,"fullName":"Jane Doe"},{"id":1,"fullName":"John Doe"}]}}"###
        );

        // Order by relation
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestView(orderBy: { children: { _count: asc } }) { id _count { children } } }"#),
          @r###"{"data":{"findManyTestView":[{"id":2,"_count":{"children":0}},{"id":3,"_count":{"children":0}},{"id":1,"_count":{"children":2}}]}}"###
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        migrate_view(&runner).await?;

        create_test_model(runner, r#"{ id: 1, firstName: "John", lastName: "Doe" }"#).await?;
        create_test_model(runner, r#"{ id: 2, firstName: "Jane", lastName: "Doe" }"#).await?;
        create_test_model(runner, r#"{ id: 3, firstName: "Bob", lastName: "Maurane" }"#).await?;

        create_child(runner, r#"{ id: 1, name: "Derek" viewId: 1 }"#).await?;
        create_child(runner, r#"{ id: 2, name: "Kevin" viewId: 1 }"#).await?;

        Ok(())
    }

    async fn migrate_view(runner: &Runner) -> TestResult<()> {
        let sql = migrate_view_sql(runner);
        run_query!(&runner, fmt_execute_raw(sql, vec![]));

        Ok(())
    }

    fn migrate_view_sql(runner: &Runner) -> &str {
        match runner.connector() {
            ConnectorTag::Postgres(_)
            | ConnectorTag::MySql(_)
            | ConnectorTag::Sqlite(_)
            | ConnectorTag::Cockroach(_)
            | ConnectorTag::Vitess(_) => {
                r#"CREATE VIEW "TestView" AS SELECT "TestModel".*, CONCAT("TestModel"."firstName", ' ', "TestModel"."lastName") as "fullName" From "TestModel""#
            }
            ConnectorTag::SqlServer(_) => todo!(),
            ConnectorTag::MongoDb(_) => unreachable!(),
        }
    }

    async fn create_test_model(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTestModel(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }

    async fn create_child(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneChild(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
