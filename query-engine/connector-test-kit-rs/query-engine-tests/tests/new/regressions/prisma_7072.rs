use query_engine_tests::*;

#[test_suite(schema(schema))]
mod filter_in {
    use indoc::indoc;
    use query_engine_tests::assert_query;

    fn schema() -> String {
        let schema = indoc! {r#"
            model Foo {
                #id(id, String, @id)
                version String
                name	  String
                bar		  Bar?

                @@unique([id, version])
            }

            model Bar {
                #id(id, String, @id)
                name		String
                fooId		String
                version String
                foo			Foo	@relation(fields: [fooId, version], references: [id, version])
            }
        "#};

        schema.to_owned()
    }

    #[connector_test]
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
}
