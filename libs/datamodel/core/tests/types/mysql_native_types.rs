use crate::common::*;
use datamodel::{ast, diagnostics::DatamodelError};

#[test]
fn should_fail_on_native_type_text_with_unique_attribute() {
    let dml = r#"
        datasource db {
          provider = "mysql"
          url = "mysql://"
          previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id     Int    @id
            bigInt String @db.Text @unique
        }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_connector_error(
        "Native type Text can not be unique in MySQL.",
        ast::Span::new(199, 230),
    ));
}
