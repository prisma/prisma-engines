use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(KingbaseOracle))]
async fn oracle_check_constraints_are_reported(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE checked_values (
            id INTEGER PRIMARY KEY,
            value NUMBER(10, 0) NOT NULL,
            CONSTRAINT checked_values_positive CHECK (value > 0)
        );
        "#,
    )
    .await;

    let datamodel = api.introspect().await?;
    assert!(
        datamodel.contains(
            "/// This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/check-constraints for more info."
        ),
        "{datamodel}"
    );
    assert!(datamodel.contains("model checked_values"), "{datamodel}");

    let warnings = api.introspection_warnings().await?;
    assert!(warnings.contains("Model: \"checked_values\", constraint: \"checked_values_positive\""));

    Ok(())
}
