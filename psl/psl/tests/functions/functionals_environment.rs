#[test]
fn skipping_of_env_vars() {
    for provider in ["postgresql", "kingbase-mysql", "kingbase-oracle"] {
        let dml = format!(
            r#"
        datasource db {{
            provider = "{provider}"
        }}

        model User {{
            id   Int      @id
            tags String[]
        }}
    "#
        );

        let dml = if provider == "postgresql" {
            dml
        } else {
            dml.replace("String[]", "String")
        };

        // must not fail without env var
        psl::parse_schema_without_extensions(&dml).unwrap();
    }
}
