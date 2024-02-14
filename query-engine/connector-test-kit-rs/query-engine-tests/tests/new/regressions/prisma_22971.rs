use query_engine_tests::*;

#[test_suite(schema(schema))]
mod tests {
    fn schema() -> String {
        indoc! {
            r#"
            model Parent {
                #id(id, Int, @id)
                children Child[]
            }

            model Child {
                #id(id, Int, @id)
                parentId Int    @map("parent_id")
                parent   Parent @relation(fields: [parentId], references: [id])
            }
            "#
        }
        .to_owned()
    }

    #[connector_test]
    async fn supports_mapped_parent_id(runner: Runner) -> TestResult<()> {
        let result = run_query!(
            runner,
            r#"
            mutation {
                createOneParent(
                    data: {
                        id: 1,
                        children: {
                            create: { id: 1 }
                        }
                    }
                ) {
                    children {
                        id
                        parentId
                    }
                }
            }
            "#
        );

        insta::assert_snapshot!(result, @r###"{"data":{"createOneParent":{"children":[{"id":1,"parentId":1}]}}}"###);

        Ok(())
    }
}
