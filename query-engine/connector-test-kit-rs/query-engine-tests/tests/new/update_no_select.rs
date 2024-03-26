use query_engine_tests::*;

#[test_suite]
mod update_with_no_select {
    pub fn occ_simple() -> String {
        include_str!("occ_simple.prisma").to_owned()
    }

    #[connector_test(schema(occ_simple), exclude(Sqlite("cfd1")))]
    async fn update_with_no_select(mut runner: Runner) -> TestResult<()> {
        let create_one_resource = r#"
        mutation {
            createOneResource(data: {id: 1}) {
              id
            }
          }"#;

        runner.query(create_one_resource).await.unwrap().to_json_value();

        let logs = runner.get_logs().await;

        let has_select = logs.contains(&"SELECT".to_string());

        assert!(!has_select);

        Ok(())
    }
}
