use crate::common::*;
use dml::field::FieldArity;

#[test]
fn parse_unsupported_types() {
    let dml = r#"
    datasource db {
            provider        = "postgres"
            url             = "postgresql://"
    }
    
    model User {
        id           Int    @id
        point        Unsupported("point")
        ip           Unsupported("ip4r")?
        with_space   Unsupported("something weird with spaces")[]
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model
        .assert_has_scalar_field("point")
        .assert_unsupported_type("point")
        .assert_arity(&FieldArity::Required);
    user_model
        .assert_has_scalar_field("ip")
        .assert_unsupported_type("ip4r")
        .assert_arity(&FieldArity::Optional);
    user_model
        .assert_has_scalar_field("with_space")
        .assert_unsupported_type("something weird with spaces")
        .assert_arity(&FieldArity::List);
}
