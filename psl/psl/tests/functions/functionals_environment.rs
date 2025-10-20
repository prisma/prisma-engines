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
    psl::parse_schema_without_extensions(dml).unwrap();
}

#[test]
fn env_function_without_default_var_is_missing() {
    let dml = r#"
        datasource db {
            provider = "postgresql"
            url      = env("MISSING_ENV_VAR")
        }
    "#;

    let config = psl::parse_schema_without_extensions(dml).unwrap();
    let error = config.configuration.datasources[0].load_url(|_| None).unwrap_err();
    let expected_error = "Environment variable not found: MISSING_ENV_VAR.";
    assert!(error.to_pretty_string("schema.prisma", dml).contains(expected_error));
}

#[test]
fn env_function_with_env_var_and_no_default() {
    unsafe { std::env::set_var("DB_URL_SET", "postgresql://real@localhost/realdb") };
    let dml = r#"
        datasource db {
            provider = "postgresql"
            url      = env("DB_URL_SET")
        }
    "#;

    let schema = psl::parse_schema_without_extensions(dml).unwrap();
    let url = schema.configuration.datasources.get(0).unwrap().load_url(|key| std::env::var(key).ok()).unwrap();
    assert_eq!(url, "postgresql://real@localhost/realdb");
    unsafe { std::env::remove_var("DB_URL_SET") };
}


#[test]
fn env_function_with_default_value() {
    // Environment variable is present, default should be ignored.
    unsafe { std::env::set_var("POSTGRES_URL", "postgresql://env_user@localhost/env_db") };
    let dml_present = r#"
        datasource db {
            provider = "postgresql"
            url      = env("POSTGRES_URL", default: "postgresql://default_user@localhost/default_db")
        }
    "#;
    let schema_present = psl::parse_schema_without_extensions(dml_present).unwrap();
    let url_present = schema_present.configuration.datasources.get(0).unwrap().load_url(|key| std::env::var(key).ok()).unwrap();
    assert_eq!(url_present, "postgresql://env_user@localhost/env_db");
    unsafe { std::env::remove_var("POSTGRES_URL") };

    // Environment variable is absent, default should be used.
    let dml_absent_with_default = r#"
        datasource db {
            provider = "postgresql"
            url      = env("NON_EXISTENT_URL", default: "postgresql://default_user@localhost/default_db")
        }
    "#;
    let schema_absent_with_default = psl::parse_schema_without_extensions(dml_absent_with_default).unwrap();
    let url_absent_with_default = schema_absent_with_default.configuration.datasources.get(0).unwrap().load_url(|_| None).unwrap();
    assert_eq!(url_absent_with_default, "postgresql://default_user@localhost/default_db");
}

#[test]
fn env_function_with_default_value_for_direct_url() {
    // Case 1: Environment variable is present, default should be ignored.
    unsafe { std::env::set_var("DIRECT_DATABASE_URL_WITH_DEFAULT", "postgresql://env_direct_user@localhost/env_direct_db") };
    let dml_present = r#"
        datasource db {
            provider = "postgresql"
            url = "postgresql://dummy@localhost/dummy"
            directUrl = env("DIRECT_DATABASE_URL_WITH_DEFAULT", default: "postgresql://default_direct_user@localhost/default_direct_db")
        }
    "#;
    let schema_present = psl::parse_schema_without_extensions(dml_present).unwrap();
    let direct_url_present = schema_present.configuration.datasources.get(0).unwrap().load_direct_url(|key| std::env::var(key).ok()).unwrap();
    assert_eq!(direct_url_present, "postgresql://env_direct_user@localhost/env_direct_db");
    unsafe { std::env::remove_var("DIRECT_DATABASE_URL_WITH_DEFAULT") };

    // Case 2: Environment variable is absent, default should be used.
    let dml_absent_with_default = r#"
        datasource db {
            provider = "postgresql"
            url = "postgresql://dummy@localhost/dummy"
            directUrl = env("NON_EXISTENT_DIRECT_URL", default: "postgresql://default_direct_user@localhost/default_direct_db")
        }
    "#;
    let schema_absent_with_default = psl::parse_schema_without_extensions(dml_absent_with_default).unwrap();
    let direct_url_absent_with_default = schema_absent_with_default.configuration.datasources.get(0).unwrap().load_direct_url(|_| None).unwrap();
    assert_eq!(direct_url_absent_with_default, "postgresql://default_direct_user@localhost/default_direct_db");

}