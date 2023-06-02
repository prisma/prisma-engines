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

    let schema = psl::parse_schema(dml).unwrap();

    let user = schema.assert_has_model("User");
    user.assert_mapped_name("user");
    user.assert_has_scalar_field("firstName")
        .assert_mapped_name("first_name");

    let post = schema.assert_has_model("Post");
    post.assert_mapped_name("posti");
    post.assert_has_scalar_field("text").assert_mapped_name("post_text");
}

#[test]
fn map_on_composite_type_field() {
    let dml = r#"
        datasource db {
            provider = "mongodb"
            url = "mongodb://"
        }

        type Address {
            fullName String @map("full_name")
        }
   "#;

    psl::parse_schema(dml)
        .unwrap()
        .assert_has_type("Address")
        .assert_has_scalar_field("fullName")
        .assert_mapped_name("full_name");
}
