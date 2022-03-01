use crate::common::parse;

#[test]
fn skipping_of_env_vars() {
    let dml = r#"
        datasource db {
            provider = "postgresql"
            url      = env("POSTGRES_URL")
        }

        model User {
            id   Int      @id
            tags String[]
        }
    "#;

    // must not fail without env var
    parse(dml);
}
