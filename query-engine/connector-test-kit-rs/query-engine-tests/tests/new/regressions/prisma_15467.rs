use query_engine_tests::*;

#[test_suite(schema(schema), only(MongoDb))]
mod mongodb {
    use indoc::indoc;

    fn schema() -> String {
        let schema = indoc! {
            r#"
            model Standing {
                id String @id @default(auto()) @map("_id") @test.ObjectId
                leagueId Int
                teamId Int
                awayLosses Int
            }
            "#
        };
        schema.to_owned()
    }

    #[connector_test]
    async fn update_many_log_output(mut runner: Runner) -> TestResult<()> {
        let insert_one_standing = r#"
        mutation {
            createOneStanding(data:{leagueId: 0, teamId: 0, awayLosses: 0}) {
                id
            }
        }"#;

        let res = run_query_json!(&runner, insert_one_standing);
        let object_id = &res["data"]["createOneStanding"]["id"];
        let _ = runner.get_logs().await;

        run_query!(
            &runner,
            format!(
                r#"
                mutation {{
                    updateManyStanding(data:{{awayLosses:{{set: 0}}, teamId:{{set: 972030012}}, leagueId:{{set: 2363725}}}}, where:{{id: {{equals: {object_id} }}}}) {{
                        count
                    }}
                }}
                "#
            )
        );
        let logs = runner.get_logs().await;
        let last_log_line = logs.last().unwrap();
        let query = format!(
            r#"
db.Standing.updateMany({{
    _id: {{
        $in: [
            ObjectId({object_id}),
        ],
    }},
}},[
{{
    $set: {{
        leagueId: {{
            $literal: 2363725,
        }},
    }},
}},
{{
    $set: {{
        teamId: {{
            $literal: 972030012,
        }},
    }},
}},
{{
    $set: {{
        awayLosses: {{
            $literal: 0,
        }},
    }},
}}])"#
        );

        let expected_query = query.trim();
        assert!(
            last_log_line.contains(expected_query),
            r#"{last_log_line} should have contained {expected_query}"#,
        );

        // Piggybacking assertion reproducing https://github.com/prisma/prisma/issues/14378
        let expected_duration_field = "duration_ms";
        assert!(
            last_log_line.contains(expected_duration_field),
            r#"{last_log_line} should have contained {expected_duration_field}"#
        );

        Ok(())
    }
}
