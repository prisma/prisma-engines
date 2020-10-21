use crate::common::*;
use dml::default_value::DefaultValue;
use native_types::PostgresType;
use prisma_value::PrismaValue;

#[test]
fn should_handle_default_on_byte_a() {
    let dml = r#"
        datasource pg {
          provider = "postgres"
          url = "postgresql://"
        }

        generator client {
          provider = "prisma-client-js"
          previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id     Int    @id
            foo    Bytes @pg.ByteA @default("aGVsbG8=")
        }
    "#;

    let datamodel = parse(dml);

    let user_model = datamodel.assert_has_model("Blog");

    user_model
        .assert_has_scalar_field("foo")
        .assert_default_value(DefaultValue::Single(PrismaValue::Bytes(vec![104, 101, 108, 108, 111])));

    let sft = user_model.assert_has_scalar_field("foo").assert_native_type();

    let postgres_type: PostgresType = sft.deserialize_native_type();
    assert_eq!(postgres_type, PostgresType::ByteA);
}
