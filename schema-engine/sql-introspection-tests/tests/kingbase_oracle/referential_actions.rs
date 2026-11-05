use sql_introspection_tests::test_api::*;
use test_macros::test_connector;

#[test_connector(tags(KingbaseOracle))]
async fn oracle_supported_delete_actions_are_introspected(api: &mut TestApi) -> TestResult {
    api.raw_cmd(
        r#"
        CREATE TABLE parent (
            id INTEGER PRIMARY KEY
        );

        CREATE TABLE cascade_child (
            id INTEGER PRIMARY KEY,
            parent_id INTEGER NOT NULL,
            CONSTRAINT cascade_child_parent_fk
                FOREIGN KEY (parent_id) REFERENCES parent (id) ON DELETE CASCADE
        );

        CREATE TABLE restrict_child (
            id INTEGER PRIMARY KEY,
            parent_id INTEGER NOT NULL,
            CONSTRAINT restrict_child_parent_fk
                FOREIGN KEY (parent_id) REFERENCES parent (id) ON DELETE RESTRICT
        );

        CREATE TABLE default_child (
            id INTEGER PRIMARY KEY,
            parent_id INTEGER NOT NULL DEFAULT 0,
            CONSTRAINT default_child_parent_fk
                FOREIGN KEY (parent_id) REFERENCES parent (id) ON DELETE SET DEFAULT
        );
        "#,
    )
    .await;

    let result = api.introspect_dml().await?;

    for expected in [
        "onDelete: Cascade",
        "onDelete: SetDefault",
        "onUpdate: NoAction",
        "model restrict_child",
    ] {
        assert!(result.contains(expected), "missing `{expected}` in:\n{result}");
    }

    // RESTRICT is Prisma's required-relation default and is intentionally omitted from rendered PSL.
    assert!(!result.contains("onDelete: Restrict"), "{result}");

    Ok(())
}
