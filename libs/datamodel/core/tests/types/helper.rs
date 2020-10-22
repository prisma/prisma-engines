use crate::common::*;

pub fn test_native_types_compatibility(datamodel: &str, error_msg: &str, datasource: &str) {
    let dml = format!(
        r#"
    {datasource}

    generator js {{
            provider = "prisma-client-js"
            previewFeatures = ["nativeTypes"]
        }}

    {datamodel}
    "#,
        datasource = datasource,
        datamodel = datamodel,
    );

    let error = parse_error(&dml);

    error.assert_is_message(error_msg);
}

pub fn test_native_types_with_field_attribute_support(
    native_type: &str,
    scalar_type: &str,
    attribute_name: &str,
    error_msg: &str,
    datasource: &str,
) {
    let id_field = if attribute_name == "id" {
        ""
    } else {
        "id     Int    @id"
    };
    let dml = format!(
        r#"
    model Blog {{
      {id_field}
      bigInt {scalar_type} @db.{native_type} @{attribute_name}
    }}
    "#,
        id_field = id_field,
        native_type = native_type,
        scalar_type = scalar_type,
        attribute_name = attribute_name
    );

    test_native_types_compatibility(&dml, &error_msg, datasource);
}
