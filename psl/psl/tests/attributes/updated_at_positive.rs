use psl::parser_database::ScalarType;

use crate::common::*;

#[test]
fn should_apply_updated_at_attribute() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          lastSeen DateTime @updatedAt
        }
    "#};

    let schema = psl::parse_schema(dml).unwrap();
    let model = schema.assert_has_model("User");

    model
        .assert_has_scalar_field("lastSeen")
        .assert_scalar_type(ScalarType::DateTime)
        .assert_is_updated_at();
}
