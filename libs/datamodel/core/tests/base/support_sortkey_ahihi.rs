use crate::common::*;

#[test]
fn parse_basic_model_with_ahihi_attribute() {
    let dml = r#"
    model User {
        id Int @id @ahihi
        firstName String
        lastName String
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model
        .assert_has_scalar_field("firstName")
        .assert_base_type(&ScalarType::String);
    user_model
        .assert_has_scalar_field("lastName")
        .assert_base_type(&ScalarType::String);
}
