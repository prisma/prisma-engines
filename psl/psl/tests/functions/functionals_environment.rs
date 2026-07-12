#[test]
fn skipping_of_env_vars() {
    let dml = r#"
        datasource db {
            provider = "postgresql"
        }

        model User {
            id   Int      @id
            tags String[]
        }
    "#;

    // must not fail without env var
    psl::parse_schema_without_extensions(dml).unwrap();
}
