use query_engine_tests::test_suite;

#[test_suite(capabilities(MultiSchema))]
mod multi_schema {
    use query_engine_tests::*;

    pub fn multi_schema_simple() -> String {
        let schema = indoc! {
            r#"
            model TestModel {
                #id(id, Int, @id)
                field String?
                @@schema("schema1")
            }
            
            model TestModel2 {
                #id(id, Int, @id)
                number Int
                @@schema("schema2")
            }
            
            "#
        };

        schema.to_owned()
    }

    pub fn multi_schema_implicit_m2m() -> String {
        let schema = indoc! {
            r#"
                model Loop {
                    id Int @id
                    fruits Fruit[]
                    @@schema("shapes")
                }

                model Fruit {
                    id Int @id
                    loops Loop[]
                    @@schema("objects")
                }
            "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(multi_schema_simple), db_schemas("schema1", "schema2"))]
    async fn crud_simple(runner: Runner) -> TestResult<()> {
        // CREATE
        runner
            .query(r#"mutation { createOneTestModel(data: { id: 1, field: "test1" }) { id } }"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneTestModel2(data: { id: 1, number: 1 }) { id } }"#)
            .await?
            .assert_success();

        // READ
        assert_query!(
            runner,
            "query { findFirstTestModel(where: { id: 1 }) { id }}",
            r#"{"data":{"findFirstTestModel":{"id":1}}}"#
        );

        assert_query!(
            runner,
            "query { findFirstTestModel2(where: { id: 1 }) { id, number }}",
            r#"{"data":{"findFirstTestModel2":{"id":1,"number":1}}}"#
        );

        // UPDATE
        assert_query!(
            runner,
            r#"mutation { updateOneTestModel(where: { id: 1 }, data: { field: "two" }) { id } }"#,
            r#"{"data":{"updateOneTestModel":{"id":1}}}"#
        );

        assert_query!(
            runner,
            r#"mutation { updateOneTestModel2(where: { id: 1 }, data: { number: 2 }) { id } }"#,
            r#"{"data":{"updateOneTestModel2":{"id":1}}}"#
        );

        assert_query!(
            runner,
            "query { findFirstTestModel(where: { id: 1 }) { id, field }}",
            r#"{"data":{"findFirstTestModel":{"id":1,"field":"two"}}}"#
        );

        assert_query!(
            runner,
            "query { findFirstTestModel2(where: { id: 1 }) { id, number }}",
            r#"{"data":{"findFirstTestModel2":{"id":1,"number":2}}}"#
        );

        // DELETE

        assert_query!(
            runner,
            "mutation { deleteOneTestModel(where: {id: 1}) { id } }",
            r#"{"data":{"deleteOneTestModel":{"id":1}}}"#
        );

        assert_query!(
            runner,
            "mutation { deleteOneTestModel2(where: {id: 1}) { id } }",
            r#"{"data":{"deleteOneTestModel2":{"id":1}}}"#
        );

        assert_query!(
            runner,
            "query { findFirstTestModel(where: { id: 1 }) { id, field }}",
            r#"{"data":{"findFirstTestModel":null}}"#
        );

        assert_query!(
            runner,
            "query { findFirstTestModel2(where: { id: 1 }) { id, number }}",
            r#"{"data":{"findFirstTestModel2":null}}"#
        );

        Ok(())
    }

    #[connector_test(schema(multi_schema_simple), db_schemas("schema1", "schema2"))]
    async fn crud_many_simple(runner: Runner) -> TestResult<()> {
        // CREATE

        runner
            .query(
                r#"mutation {
                    createManyTestModel(data: [
                        { id: 1, field: "1" },
                        { id: 2, field: "2" },
                        { id: 3, field: "3" }
                    ]) { count }
                }"#,
            )
            .await?
            .assert_success();

        // READ
        assert_query!(
            runner,
            "query { findManyTestModel(where: { id: {gt: 0} }) { id }}",
            r#"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3}]}}"#
        );

        // UPDATE
        assert_query!(
            runner,
            r#"mutation { updateManyTestModel(where: { id: {gt: 0}}, data: { field: "two" }) { count } }"#,
            r#"{"data":{"updateManyTestModel":{"count":3}}}"#
        );

        assert_query!(
            runner,
            r#"query { findManyTestModel(where: { field: "two" }) { id }}"#,
            r#"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3}]}}"#
        );

        // DELETE
        assert_query!(
            runner,
            r#"mutation { deleteManyTestModel(where: {field: "two"}) { count } }"#,
            r#"{"data":{"deleteManyTestModel":{"count":3}}}"#
        );

        assert_query!(
            runner,
            r#"query { findManyTestModel(where: { field: "two" }) { id }}"#,
            r#"{"data":{"findManyTestModel":[]}}"#
        );

        Ok(())
    }

    pub fn multi_schema_relations() -> String {
        let schema = indoc! {
            r#"
            model ChildModel {
                #id(id, Int, @id)
                field String?
                parent   ParentModel @relation(fields: [parent_id], references: [id], onDelete: Cascade)
                parent_id Int
                @@schema("schema1")
              }

            model ParentModel {
                #id(id, Int, @id)
                number Int
                @@schema("schema2")
                children  ChildModel[]
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(multi_schema_relations), db_schemas("schema1", "schema2"))]
    async fn crud_relations(runner: Runner) -> TestResult<()> {
        // CREATE
        runner
            .query(
                r#"
                mutation {
                    createOneParentModel(data: {
                        id: 1,
                        number: 1,
                        children: {
                            create: [
                                { id: 1, field: "c1" }
                            ]
                        }
                   }) {
                        id,
                        children {
                            id,
                            field
                        }
                    }
                }"#,
            )
            .await?
            .assert_success();

        // READ
        assert_query!(
            runner,
            "query { findFirstParentModel(where: {id: 1}) { id, children { id, field } } }",
            r#"{"data":{"findFirstParentModel":{"id":1,"children":[{"id":1,"field":"c1"}]}}}"#
        );

        assert_query!(
            runner,
            "query{ findFirstChildModel(where: {id: 1}) { id, parent {id, number} } }",
            r#"{"data":{"findFirstChildModel":{"id":1,"parent":{"id":1,"number":1}}}}"#
        );

        assert_query!(
            runner,
            "query { findManyParentModel(where: {id: 1}) { id, children { id, field } } }",
            r#"{"data":{"findManyParentModel":[{"id":1,"children":[{"id":1,"field":"c1"}]}]}}"#
        );

        // UPDATE
        runner
            .query(
                r#"
                    mutation {
                        updateOneParentModel(where: {id: 1}, data: {
                        number: 2,
                        children: {
                        create: {
                            id: 2, field: "c2"
                        }
                        }

                    })  {
                        id
                    }
                    }
        "#,
            )
            .await?
            .assert_success();

        assert_query!(
            runner,
            "query { findFirstParentModel(where: {id: 1}) { id, number, children { id, field } } }",
            r#"{"data":{"findFirstParentModel":{"id":1,"number":2,"children":[{"id":1,"field":"c1"},{"id":2,"field":"c2"}]}}}"#
        );

        // DELETE
        runner
            .query(
                r#"mutation {
                    deleteOneParentModel(where: {id: 1}) {
                     id
                   }
                }"#,
            )
            .await?
            .assert_success();

        assert_query!(
            runner,
            "query { findFirstParentModel(where: {id: 1}) { id, children { id, field } } }",
            r#"{"data":{"findFirstParentModel":null}}"#
        );

        assert_query!(
            runner,
            "query{ findFirstChildModel(where: {id: 1}) { id, parent {id, number} } }",
            r#"{"data":{"findFirstChildModel":null}}"#
        );

        Ok(())
    }

    #[connector_test(schema(multi_schema_relations), db_schemas("schema1", "schema2"))]
    async fn create_and_get_many_relations(runner: Runner) -> TestResult<()> {
        runner
            .query(
                r#"
                mutation {
                    createOneParentModel(data: {
                        id: 1,
                        number: 1,
                        children: {
                            create: [
                                { id: 1, field: "c1" },
                                { id: 2, field: "c2" }
                            ]
                        }
                   }) {
                        id,
                        children {
                            id,
                            field
                        }
                    }
                }"#,
            )
            .await?
            .assert_success();

        assert_query!(
            runner,
            "query { findManyParentModel(where: {id: {gt: 0}}) { id, children { id} } }",
            r#"{"data":{"findManyParentModel":[{"id":1,"children":[{"id":1},{"id":2}]}]}}"#
        );

        assert_query!(
            runner,
            "query{ findManyChildModel(where: {id: {gt: 0}}) { id, parent { id } } }",
            r#"{"data":{"findManyChildModel":[{"id":1,"parent":{"id":1}},{"id":2,"parent":{"id":1}}]}}"#
        );

        Ok(())
    }

    pub fn multi_schema_many_to_many_relations() -> String {
        let schema = indoc! {
            r#"
            model Post {
                #id(id, Int, @id)
                title  String
                categories CategoriesOnPosts[]
                @@schema("schema1")
              }
              
              model Category {
                #id(id, Int, @id)
                name  String
                posts CategoriesOnPosts[]
                @@schema("schema2")
              }
              
              model CategoriesOnPosts {
                post       Post     @relation(fields: [postId], references: [id])
                postId     Int 
                category   Category @relation(fields: [categoryId], references: [id])
                categoryId Int 
                tmp Int?
                @@schema("schema3")
              
                @@id([postId, categoryId])
              }
            "#
        };

        schema.to_owned()
    }

    #[connector_test(
        schema(multi_schema_many_to_many_relations),
        db_schemas("schema1", "schema2", "schema3")
    )]
    async fn create_and_get_many_to_many_relations(runner: Runner) -> TestResult<()> {
        runner
            .query(
                r#"
                mutation {
                    createManyPost(data: [
                        { id: 1, title: "p1" },
                        { id: 2, title: "p2" }
                    ]) {
                      count
                    }
                }"#,
            )
            .await?
            .assert_success();

        runner
            .query(
                r#"
                mutation {
                    createManyCategory(data: [
                        { id: 1, name: "c1" },
                        { id: 2, name: "c2" },
                        { id: 3, name: "c3" }
                    ]) {
                      count
                    }
                }"#,
            )
            .await?
            .assert_success();

        runner
            .query(
                r#"
                mutation {
                    createManyCategoriesOnPosts(data: [
                        { postId: 1, categoryId: 1 },
                        { postId: 1, categoryId: 2 },
                        { postId: 1, categoryId: 3 },
                        { postId: 2, categoryId: 2 },
                        { postId: 2, categoryId: 3 },
                    ]) {
                      count
                    }
                }"#,
            )
            .await?
            .assert_success();

        insta::assert_snapshot!(
          run_query!(&runner, r#"
                query {
                    findManyCategoriesOnPosts(orderBy: [{ postId: asc }, { categoryId: asc }], where: {postId: {gt: 0}}) {
                      category {
                        name
                      },
                      post {
                        title
                      }
                    }
                  }
                "#),
          @r###"{"data":{"findManyCategoriesOnPosts":[{"category":{"name":"c1"},"post":{"title":"p1"}},{"category":{"name":"c2"},"post":{"title":"p1"}},{"category":{"name":"c3"},"post":{"title":"p1"}},{"category":{"name":"c2"},"post":{"title":"p2"}},{"category":{"name":"c3"},"post":{"title":"p2"}}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"
            query {
                findManyPost(where: {id: 1}) {
                  id,
                  title,
                  categories {
                    category {
                      name,
                      id
                    }
                  }
                }
              }"#),
          @r###"{"data":{"findManyPost":[{"id":1,"title":"p1","categories":[{"category":{"name":"c1","id":1}},{"category":{"name":"c2","id":2}},{"category":{"name":"c3","id":3}}]}]}}"###
        );

        Ok(())
    }

    fn test_filter_in_schema() -> String {
        let schema = indoc! {r#"
            model Foo {
                #id(id, String, @id)
                version String
                name	  String
                bar		  Bar?
                @@schema("schema2")
                @@unique([id, version])
            }

            model Bar {
                #id(id, String, @id)
                name		String
                fooId		String
                version     String
                foo			Foo	@relation(fields: [fooId, version], references: [id, version])
                @@schema("schema1")
                @@unique([fooId, version])
            }
        "#};

        schema.to_owned()
    }

    #[connector_test(schema(test_filter_in_schema), db_schemas("schema1", "schema2"))]
    async fn test_filter_in(runner: Runner) -> TestResult<()> {
        runner
            .query(
                r#"
                mutation {
                    createOneFoo(data: {
                        id: "1"
                        version: "a"
                        name: "first foo"
                        bar: {
                            create: {
                                id: "1"
                                name: "first bar"
                            }
                        }
                    }) { id }
                }"#,
            )
            .await?
            .assert_success();

        runner
            .query(
                r#"
                mutation {
                    createOneFoo(data: {
                        id: "2"
                        version: "a"
                        name: "second foo"
                    }) { id }
                }"#,
            )
            .await?
            .assert_success();

        assert_query!(
            runner,
            "query { findManyFoo(where: { bar: { is: null } }) { id } }",
            r#"{"data":{"findManyFoo":[{"id":"2"}]}}"#
        );

        Ok(())
    }

    #[connector_test(schema(multi_schema_implicit_m2m), db_schemas("shapes", "objects"))]
    async fn implicit_m2m_simple(runner: Runner) -> TestResult<()> {
        let result = runner
            .query(r#"mutation { createOneFruit(data: { id: 1, loops: { create: [{ id: 11 }, { id: 12 }] }}) { id loops { id } } }"#)
            .await?;
        result.assert_success();
        let result = result.to_string();
        assert_eq!(
            result,
            "{\"data\":{\"createOneFruit\":{\"id\":1,\"loops\":[{\"id\":11},{\"id\":12}]}}}"
        );
        Ok(())
    }
}
