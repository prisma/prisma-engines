use crate::common::*;
use datamodel::{ast, diagnostics::DatamodelError};
use dml::scalars::ScalarType;
use native_types::{MySqlType, PostgresType};

#[test]
fn should_fail_on_native_type_decimal_when_scale_is_bigger_than_precision() {
    let dml = r#"
        datasource db {
          provider = "postgres"
          url      = "postgresql://"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id     Int   @id
            dec Decimal @db.Decimal(2, 4)
        }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_connector_error(
        "The scale must not be larger than the precision for the Decimal native type in Postgres.",
        ast::Span::new(289, 319),
    ));
}

#[test]
fn should_fail_on_native_type_numeric_when_scale_is_bigger_than_precision() {
    let dml = r#"
        datasource db {
          provider = "postgres"
          url      = "postgresql://"
        }

        generator js {
            provider = "prisma-client-js"
            previewFeatures = ["nativeTypes"]
        }

        model Blog {
            id     Int   @id
            dec Decimal @db.Numeric(2, 4)
        }
    "#;

    let error = parse_error(dml);

    error.assert_is(DatamodelError::new_connector_error(
        "The scale must not be larger than the precision for the Numeric native type in Postgres.",
        ast::Span::new(289, 319),
    ));
}

#[test]
fn xml_should_work_with_string_scalar_type() {
    let dml = format!(
        r#"
        {datasource}

        generator js {{
            provider = "prisma-client-js"
            previewFeatures = ["nativeTypes"]
        }}

        model Blog {{
            id  Int    @id
            dec String @db.Xml
        }}
    "#,
        datasource = POSTGRES_SOURCE
    );

    let datamodel = parse(&dml);
    let user_model = datamodel.assert_has_model("Blog");
    let sft = user_model.assert_has_scalar_field("dec").assert_native_type();

    let postgres_tpe: PostgresType = sft.deserialize_native_type();
    assert_eq!(postgres_tpe, PostgresType::Xml);
}
