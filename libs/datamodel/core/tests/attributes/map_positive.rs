use crate::common::*;

#[test]
fn map_attribute() {
    let dml = r#"
    model User {
        id Int @id
        firstName String @map("first_name")

        @@map("user")
    }

    model Post {
        id Int @id
        text String @map(name: "post_text")

        @@map(name: "posti")
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User").assert_with_db_name("user");
    user_model
        .assert_has_scalar_field("firstName")
        .assert_with_db_name("first_name");

    let post_model = schema.assert_has_model("Post").assert_with_db_name("posti");
    post_model
        .assert_has_scalar_field("text")
        .assert_with_db_name("post_text");
}

#[test]
fn map_on_composite_type_field() {
    let dml = r#"
        datasource db {
            provider = "mongodb"
            url = "mongodb://"
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["mongoDb"]
        }


        type Address {
            fullName String @map("full_name")
        }
   "#;

    let schema = parse(dml);
    let address_type = &schema.composite_types[0];
    assert_eq!(address_type.fields[0].database_name.as_deref(), Some("full_name"));
}
