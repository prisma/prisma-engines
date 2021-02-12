use std::sync::Arc;

use super::helpers::*;
use crate::dmmf::schema::*;
use query_core::PRISMA_NAMESPACE;
use serial_test::serial;

// Tests in this file run serially because the function `get_query_schema` depends on setting an env var.

#[test]
#[serial]
fn nullable_fields_should_be_nullable_in_group_by_output_types() {
    let dm = r#"
        datasource pg {
            provider = "postgresql"
            url = "postgresql://"
        }

        model Blog {
            id           String @id
            optional_string   String?
            required_string   String
            optional_int      Int?
            required_int      Int
        }
    "#;
    let (query_schema, datamodel) = get_query_schema(dm);
    let dmmf = crate::dmmf::render_dmmf(&datamodel, Arc::new(query_schema));
    let group_by_output_type = find_output_type(&dmmf, PRISMA_NAMESPACE, "BlogGroupByOutputType");

    iterate_output_type_fields(group_by_output_type, &dmmf, &|field, parent_type| {
        let field_in_nested_type = parent_type.name != "BlogGroupByOutputType";
        let is_nullable = field.is_nullable;

        match (field.output_type.location, field_in_nested_type) {
            (TypeLocation::Scalar, false) => match field.name.as_str() {
                "id" => assert_eq!(is_nullable, false),
                "optional_string" => assert_eq!(is_nullable, true),
                "required_string" => assert_eq!(is_nullable, false),
                "optional_int" => assert_eq!(is_nullable, true),
                "required_int" => assert_eq!(is_nullable, false),
                _ => (),
            },
            (TypeLocation::Scalar, true) => match field.name.as_str() {
                "id" => assert_eq!(is_nullable, true),
                "optional_string" => assert_eq!(is_nullable, true),
                // non-numerical fields in aggregation types should always be nullable
                "required_string" => assert_eq!(is_nullable, true),
                "optional_int" => assert_eq!(is_nullable, true),
                "required_int" => assert_eq!(is_nullable, false),
                _ => (),
            },
            _ => (),
        }
    });
}
