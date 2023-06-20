use indoc::indoc;
use query_engine_tests::*;

#[test_suite]
mod relation_filter {
    fn one_to_one_schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              childId Int? @unique
              child Child? @relation(fields:[childId], references: [id])
            }
            model Child {
              #id(id, Int, @id)
              string1 String
              string2 String
              test TestModel?
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(one_to_one_schema))]
    async fn ensure_scalar_filters_can_run(runner: Runner) -> TestResult<()> {
        // Scalar filters are already tested independently and traversing relations eventually go through the same code paths when rendering scalar filters.
        // The following assertions simply ensures that all those queries are running fine and thus that the referenced fields are properly aliased to the joins.
        run_query!(
            runner,
            r#"{ findManyTestModel(where: { child: { string1: { gt: { _ref: "string2", _container: "Child" } } } }) { id } }"#
        );
        run_query!(
            runner,
            r#"{ findManyTestModel(where: { child: { string1: { gte: { _ref: "string2", _container: "Child" } } } }) { id } }"#
        );
        run_query!(
            runner,
            r#"{ findManyTestModel(where: { child: { string1: { lt: { _ref: "string2", _container: "Child" } } } }) { id } }"#
        );
        run_query!(
            runner,
            r#"{ findManyTestModel(where: { child: { string1: { lte: { _ref: "string2", _container: "Child" } } } }) { id } }"#
        );
        run_query!(
            runner,
            r#"{ findManyTestModel(where: { child: { string1: { contains: { _ref: "string2", _container: "Child" } } } }) { id } }"#
        );
        run_query!(
            runner,
            r#"{ findManyTestModel(where: { child: { string1: { startsWith: { _ref: "string2", _container: "Child" } } } }) { id } }"#
        );
        run_query!(
            runner,
            r#"{ findManyTestModel(where: { child: { string1: { endsWith: { _ref: "string2", _container: "Child" } } } }) { id } }"#
        );

        Ok(())
    }

    fn one_to_one_list_schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
            #id(id, Int, @id)
            childId Int? @unique
            child Child? @relation(fields:[childId], references: [id])
          }
          model Child {
            #id(id, Int, @id)
            string1 String
            string2 String[]
            test TestModel?
          }
          "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(one_to_one_list_schema), capabilities(ScalarLists))]
    async fn ensure_scalar_list_filters_can_run(runner: Runner) -> TestResult<()> {
        // Scalar list filters are already tested independently and traversing relations eventually go through the same code paths when rendering scalar list filters.
        // The following assertions simply ensures that all those queries are running fine and thus that the referenced fields are properly aliased to the joins.
        run_query!(
            runner,
            r#"{ findManyTestModel(where: { child: { string1: { in: { _ref: "string2", _container: "Child" } } } }) { id } }"#
        );
        run_query!(
            runner,
            r#"{ findManyTestModel(where: { child: { string1: { notIn: { _ref: "string2", _container: "Child" } } } }) { id } }"#
        );
        run_query!(
            runner,
            r#"{ findManyTestModel(where: { child: { string2: { hasSome: { _ref: "string2", _container: "Child" } } } }) { id } }"#
        );
        run_query!(
            runner,
            r#"{ findManyTestModel(where: { child: { string2: { hasEvery: { _ref: "string2", _container: "Child" } } } }) { id } }"#
        );

        Ok(())
    }

    #[connector_test(schema(one_to_one_schema))]
    async fn one_to_one(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
              id: 1,
              child: {
                create: {
                  id: 1,
                  string1: "abc",
                  string2: "abc"
                }
              }
            }"#,
        )
        .await?;
        create_row(
            &runner,
            r#"{
              id: 2,
              child: {
                create: {
                  id: 2,
                  string1: "abc",
                  string2: "bcd"
                }
              }
            }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTestModel(where: { child: { string1: { equals: { _ref: "string2", _container: "Child" } } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        Ok(())
    }

    fn one_to_many_schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
            #id(id, Int, @id)
            children Child[]
          }
          model Child {
            #id(id, Int, @id)
            string1 String
            string2 String
            testId Int?
            test TestModel? @relation(fields:[testId], references: [id])
          }
          "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(one_to_many_schema))]
    async fn one_to_many(runner: Runner) -> TestResult<()> {
        create_to_many_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTestModel(where: { children: { some: { string1: { equals: { _ref: "string2", _container: "Child" } } } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTestModel(where: { children: { none: { string1: { equals: { _ref: "string2", _container: "Child" } } } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":3}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTestModel(where: { children: { every: { string1: { equals: { _ref: "string2", _container: "Child" } } } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        Ok(())
    }

    fn many_to_many_schema() -> String {
        let schema = indoc! {
          r#"model TestModel {
            #id(id, Int, @id)
            #m2m(children, Child[], id, Int)
          }
          model Child {
            #id(id, Int, @id)
            string1 String
            string2 String
            #m2m(tests, TestModel[], id, Int)
          }
          "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(many_to_many_schema))]
    async fn many_to_many(runner: Runner) -> TestResult<()> {
        create_to_many_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTestModel(where: { children: { some: { string1: { equals: { _ref: "string2", _container: "Child" } } } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTestModel(where: { children: { none: { string1: { equals: { _ref: "string2", _container: "Child" } } } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":3}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTestModel(where: { children: { every: { string1: { equals: { _ref: "string2", _container: "Child" } } } } }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        Ok(())
    }

    fn complex_rel() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              toMany OneToMany[]
            }
            model OneToMany {
              #id(id, Int, @id)

              testId Int?
              test TestModel? @relation(fields:[testId], references: [id])

              toOneId Int? @unique
              toOne ToOne? @relation(fields: [toOneId], references: [id])
            }
            
            model ToOne {
              #id(id, Int, @id)

              string1 String
              string2 String

              toMany OneToMany?
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(complex_rel))]
    async fn complex_relation_traversal(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
              id: 1,
              toMany: {
                create: [
                  {
                    id: 1,
                    toOne: {
                      create: {
                        id: 1,
                        string1: "abc",
                        string2: "abc"
                      }
                    }
                  },
                  {
                    id: 2,
                    toOne: {
                      create: {
                        id: 2,
                        string1: "abc",
                        string2: "bcd"
                      }
                    }
                  },
                ]
              }
            }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel(where: {
            toMany: {
              some: {
                toOne: {
                  string1: { equals: { _ref: "string2", _container: "ToOne" } }
                }
              }
            }
          }) { id } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        Ok(())
    }

    async fn create_to_many_data(runner: &Runner) -> TestResult<()> {
        create_row(
            runner,
            r#"{
              id: 1,
              children: {
                create: [
                  {
                    id: 1,
                    string1: "abc",
                    string2: "abc"
                  },
                  {
                    id: 2,
                    string1: "abc",
                    string2: "abc"
                  },
                ]
              }
            }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{
              id: 2,
              children: {
                create: [
                  {
                    id: 3,
                    string1: "abc",
                    string2: "abc"
                  },
                  {
                    id: 4,
                    string1: "abc",
                    string2: "bcd"
                  },
                ]
              }
            }"#,
        )
        .await?;
        create_row(
            runner,
            r#"{
              id: 3,
              children: {
                create: [
                  {
                    id: 5,
                    string1: "bcd",
                    string2: "abc"
                  },
                  {
                    id: 6,
                    string1: "abc",
                    string2: "bcd"
                  },
                ]
              }
            }"#,
        )
        .await?;

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTestModel(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}
