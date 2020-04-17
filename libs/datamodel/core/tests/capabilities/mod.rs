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
