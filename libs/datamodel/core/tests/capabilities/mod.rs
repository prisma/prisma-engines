use crate::common::ErrorAsserts;
use crate::common::*;
use datamodel::ast::Span;
use datamodel::error::DatamodelError;

#[test]
fn enums_must_not_be_suppored_on_sqlite() {
    let dml = r#"
    datasource db {
      provider = "sqlite"
      url = "file://bla.db"
    }
    
    model Todo {
      id     Int    @id
      status Status
    }
    
    enum Status {
      DONE
      NOT_DONE
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_is(DatamodelError::new_validation_error(
        "You defined the enum `Status`. But the current connector does not support enums.",
        Span::new(162, 207),
    ));
}

#[test]
fn enums_must_not_be_supported_for_a_multi_provider_connector_that_contains_postgres_and_sqlite() {
    // Postgres supports enums but SQLite doesn't. Hence they can't be used in the following schema.
    test_enum_support(&["postgres", "sqlite"], true);
    // TODO: add more combinations
}

fn test_enum_support(providers: &[&str], should_error: bool) {
    // Postgres supports enums but SQLite doesn't. Hence they can't be used in the following schema.
    let provider_strings: Vec<_> = providers.iter().map(|x| format!("\"{}\"", x)).collect();
    let dml = format!(
        r#"
    datasource db {{
      provider = [{}]
      url = "file://bla.db"
    }}
    
    model Todo {{
      id     Int    @id
      status Status
    }}
    
    enum Status {{
      DONE
      NOT_DONE
    }}
    "#,
        provider_strings.join(",")
    );

    if should_error {
        let errors = parse_error(&dml);
        errors.assert_is(DatamodelError::new_validation_error(
            "You defined the enum `Status`. But the current connector does not support enums.",
            Span::new(175, 220), // TODO: figure out how to make this work for any span combination. Maybe just compare the resulting string. Add a new functoin to the assertions bla.
        ));
    } else {
        parse(&dml);
    }
}
