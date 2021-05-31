use query_engine_tests::*;

/// Note regarding Scala port: Lots of tests omitted (obsolete tests), added additional types.
#[test_suite(schema(schemas::common_list_types), capabilities(ScalarLists))]
mod scalar_list {
    use indoc::{formatdoc, indoc};
    use query_engine_tests::{assert_query, string_to_base64};

    #[connector_test]
    async fn empty_lists(runner: &Runner) -> TestResult<()> {
        test_data(
            runner,
            1,
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .await?;

        assert_query!(
            runner,
            indoc! {
                r#"query {
                    findManyTestModel {
                        string
                        int
                        bInt
                        float
                        decimal
                        bytes
                        bool
                        dt
                    }
                }"#
            },
            r#"{"data":{"findManyTestModel":[{"string":[],"int":[],"bInt":[],"float":[],"decimal":[],"bytes":[],"bool":[],"dt":[]}]}}"#
        );

        Ok(())
    }

    #[connector_test]
    async fn non_empty_lists(runner: &Runner) -> TestResult<()> {
        test_data(
            runner,
            1,
            vec!["test"],
            vec![1, 2, 3],
            vec!["1234", "4321"],
            vec![15.5, 666.0],
            vec![], //"12345.6789" TODO decimal inaccuracy
            vec![string_to_base64("test"), string_to_base64("test2")],
            vec![true, false, true, true],
            vec![date_iso_string(1990, 5, 2)],
        )
        .await?;

        assert_query!(
            runner,
            indoc! {
                r#"query {
                    findManyTestModel {
                        string
                        int
                        bInt
                        float
                        decimal
                        bytes
                        bool
                        dt
                    }
                }"#
            },
            r#"{"data":{"findManyTestModel":[{"string":["test"],"int":[1,2,3],"bInt":["1234","4321"],"float":[15.5,666.0],"decimal":[],"bytes":["dGVzdA==","dGVzdDI="],"bool":[true,false,true,true],"dt":["1990-05-02T00:00:00.000Z"]}]}}"#
        );

        Ok(())
    }

    async fn test_data(
        runner: &Runner,
        id: usize,
        str: Vec<&str>,
        int: Vec<i64>,
        b_int: Vec<&str>,
        float: Vec<f64>,
        decimal: Vec<&str>,
        bytes: Vec<String>,
        bool: Vec<bool>,
        dt: Vec<String>,
    ) -> TestResult<()> {
        runner
            .query(formatdoc! {r#"
                mutation {{
                    createOneTestModel(data: {{
                        id:      {}
                        string:  {{ set: [{}] }},
                        int:     {{ set: [{}] }},
                        bInt:    {{ set: [{}] }},
                        float:   {{ set: [{}] }},
                        decimal: {{ set: [{}] }},
                        bytes:   {{ set: [{}] }},
                        bool:    {{ set: [{}] }},
                        dt:      {{ set: [{}] }},
                    }}) {{
                        id
                    }}
                }}
            "#,
                id,
                enclose_all(str, "\"").join(","),
                stringify(int).join(", "),
                enclose_all(b_int, "\"").join(","),
                stringify(float).join(", "),
                enclose_all(decimal, "\"").join(","),
                enclose_all(bytes, "\"").join(","),
                stringify(bool).join(","),
                enclose_all(dt, "\"").join(","),
            })
            .await?
            .assert_success();

        Ok(())
    }
}
