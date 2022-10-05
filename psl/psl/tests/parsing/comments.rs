use crate::common::*;

#[test]
fn trailing_comments_allowed_in_configuration_blocks() {
    let schema = r#"
      datasource db {
        provider     = "postgres" // "mysql" | "sqlite" ...
        url          = env("TEST_POSTGRES_URI")
        relationMode = "prisma" // = on or set to "foreignKeys" to turn off emulation
      }

      generator js {
        provider        = "prisma-client-js" // optional
        previewFeatures = ["referentialIntegrity"] // []
      }     
    "#;
    assert_valid(schema);
}
