mod asserts;

pub(crate) use ::indoc::{formatdoc, indoc};
pub(crate) use asserts::*;
pub(crate) use expect_test::expect;

use psl::{parse_configuration_multi_file, Configuration, SourceFile};

pub(crate) fn reformat(input: &str) -> String {
    psl::reformat(input, 2).unwrap_or_else(|| input.to_owned())
}

pub(crate) fn parse_unwrap_err(schema: &str) -> String {
    psl::parse_schema(schema).map(drop).unwrap_err()
}

#[track_caller]
pub(crate) fn parse_schema(datamodel_string: &str) -> psl::ValidatedSchema {
    psl::parse_schema(datamodel_string).unwrap()
}

pub(crate) fn parse_config(schema: &str) -> Result<Configuration, String> {
    psl::parse_configuration(schema).map_err(|err| err.to_pretty_string("schema.prisma", schema))
}

#[track_caller]
pub(crate) fn parse_configuration(datamodel_string: &str) -> Configuration {
    match psl::parse_configuration(datamodel_string) {
        Ok(c) => c,
        Err(errs) => {
            panic!(
                "Configuration parsing failed\n\n{}",
                errs.to_pretty_string("", datamodel_string)
            )
        }
    }
}

#[track_caller]
pub(crate) fn render_datasources(datamodel_string: &str) -> String {
    let src = SourceFile::new_allocated(datamodel_string.to_owned().into_boxed_str().into());
    match parse_configuration_multi_file(&[("schema.prisma".into(), src)]) {
        Ok((files, config)) => psl::get_config::render_sources_to_json(&config.datasources, &files),
        Err((files, errors)) => panic!("Schema parsing failed:\n\n{}", files.render_diagnostics(&errors)),
    }
}

#[track_caller]
pub(crate) fn expect_error(schema: &str, expectation: &expect_test::Expect) {
    match psl::parse_schema(schema) {
        Ok(_) => panic!("Expected a validation error, but the schema is valid."),
        Err(err) => expectation.assert_eq(&err),
    }
}

pub(crate) fn parse_and_render_error(schema: &str) -> String {
    parse_unwrap_err(schema)
}

#[track_caller]
pub(crate) fn assert_valid(schema: &str) {
    match psl::parse_schema(schema) {
        Ok(_) => (),
        Err(err) => panic!("{err}"),
    }
}

pub(crate) const SQLITE_SOURCE: &str = r#"
    datasource db {
        provider = "sqlite"
        url      = "file:dev.db"
    }
"#;

pub(crate) const POSTGRES_SOURCE: &str = r#"
    datasource db {
        provider = "postgres"
        url      = "postgresql://localhost:5432"
    }
"#;

pub(crate) const MYSQL_SOURCE: &str = r#"
    datasource db {
        provider = "mysql"
        url      = "mysql://localhost:3306"
    }
"#;
