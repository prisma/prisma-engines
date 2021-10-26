use datamodel::parse_schema;

#[test]
fn mongodb_supports_composite_types() {
    let schema = r#"
        datasource db {
            provider = "mongodb"
            url = "mongodb://"
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["mongoDb"]
        }

        type Address {
            street String
        }
    "#;

    assert!(parse_schema(schema).is_ok());
}
