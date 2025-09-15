use indoc::indoc;
use psl::parser_database::{ExtensionTypeId, ExtensionTypes, ScalarFieldType};

use crate::{
    Provider,
    common::{DatamodelAssert, ModelAssert, ScalarFieldAssert},
    with_header,
};

#[test]
fn extension_type_reference() {
    let dml = indoc! {r#"
        model A {
          id Int   @id @map("_id")
          a Vector
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let extensions = TestExtensions {
        types: vec!["Vector".into()],
    };
    let datamodel = psl::parse_schema(schema, &extensions).unwrap();
    let model = datamodel.assert_has_model("A");

    model
        .assert_has_scalar_field("a")
        .assert_scalar_field_type(ScalarFieldType::Extension(
            extensions.extension_type_by_name("Vector").unwrap(),
        ));
}

struct TestExtensions {
    types: Vec<String>,
}

impl ExtensionTypes for TestExtensions {
    fn extension_type_by_name(&self, name: &str) -> Option<ExtensionTypeId> {
        self.types.iter().position(|t| t == name).map(ExtensionTypeId::from)
    }
}
