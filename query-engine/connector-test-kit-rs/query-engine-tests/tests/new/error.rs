use query_engine_tests::*;

#[test_suite(schema(schema))]
mod errors {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model Album {
                    #id(id, Int, @id, @unique)
                    Title String
                }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn errors_include_error_details(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"
                mutation {
                    createOneAlbum(data: {
                        id: 1,
                        Title: "I made an album"
                    }) {
                        id
                    }
                }
            "#
        );

        let res = runner
            .query(
                r#"
                mutation {
                    createOneAlbum(data: {
                        id: 1,
                        Title: "I made an album"
                    }) {
                        id
                    }
                }
            "#,
            )
            .await?;

        let resp = res.to_string();
        let errors: serde_json::Value = serde_json::from_str(&resp).unwrap();
        let error = &errors["errors"][0];
        let details = &error["user_facing_error"]["error_details"];

        assert_eq!(details["name"], "UniqueKeyViolation");
        assert_eq!(details["code"], "P2002");

        Ok(())
    }
}
