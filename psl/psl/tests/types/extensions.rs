use expect_test::expect;
use indoc::indoc;
use psl::parser_database::{ExtensionTypeEntry, ExtensionTypeId, ExtensionTypes, ScalarFieldType};

use crate::{
    Provider,
    common::{DatamodelAssert, ModelAssert, ScalarFieldAssert},
    with_header,
};

#[test]
fn accepts_extension_type_reference() {
    let dml = indoc! {r#"
        model A {
          id Int   @id @map("_id")
          a  Vector3
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let extensions = TestExtensions {
        types: vec![
            ("Vector3".into(), "vector".into(), 1, Some(vec!["3".into()])),
            ("VectorN".into(), "vector".into(), 1, None),
        ],
    };
    let datamodel = psl::parse_schema(schema, &extensions).unwrap();
    let model = datamodel.assert_has_model("A");

    model
        .assert_has_scalar_field("a")
        .assert_scalar_field_type(ScalarFieldType::Extension(
            extensions.get_by_prisma_name("Vector3").unwrap(),
        ));
}

#[test]
fn rejects_an_extension_type_marked_unsupported() {
    let dml = indoc! {r#"
        model A {
          id Int   @id @map("_id")
          a  Unsupported("vector")
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let extensions = TestExtensions {
        types: vec![("VectorN".into(), "vector".into(), 1, None)],
    };
    let datamodel = psl::parse_schema(schema, &extensions).unwrap_err();
    expect![[r#"
        [1;91merror[0m: [1mError validating: The type `Unsupported("vector")` you specified in the type definition for the field `a` is supported as a native type by Prisma. Please use the native type notation `VectorN @test.vector` for full support.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id Int   @id @map("_id")
        [1;94m13 | [0m  [1;91ma  Unsupported("vector")[0m
        [1;94m14 | [0m}
        [1;94m   | [0m
    "#]]
        .assert_eq(&datamodel.to_string());
}

#[test]
fn rejects_invalid_extension_modifier() {
    let dml = indoc! {r#"
        model A {
          id Int     @id @map("_id")
          a  Vector3 @test.vector(100)
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let extensions = TestExtensions {
        types: vec![
            ("Vector3".into(), "vector".into(), 1, Some(vec!["3".into()])),
            ("VectorN".into(), "vector".into(), 1, None),
        ],
    };
    let datamodel = psl::parse_schema(schema, &extensions).unwrap_err();
    expect![[r#"
        [1;91merror[0m: [1mNative type vector is not compatible with declared field type Vector3, expected field type VectorN.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id Int     @id @map("_id")
        [1;94m13 | [0m  a  Vector3 [1;91m@test.vector(100)[0m
        [1;94m   | [0m
    "#]]
        .assert_eq(&datamodel.to_string());
}

#[test]
fn rejects_missing_extension_modifier() {
    let dml = indoc! {r#"
        model A {
          id Int     @id @map("_id")
          a  VectorN @test.vector
        }
    "#};

    let schema = with_header(dml, Provider::Postgres, &[]);
    let extensions = TestExtensions {
        types: vec![
            ("Vector3".into(), "vector".into(), 1, Some(vec!["3".into()])),
            ("VectorN".into(), "vector".into(), 1, None),
        ],
    };
    let datamodel = psl::parse_schema(schema, &extensions).unwrap_err();
    expect![[r#"
        [1;91merror[0m: [1mFunction "vector" takes 1 arguments, but received 0.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  id Int     @id @map("_id")
        [1;94m13 | [0m  a  VectorN [1;91m@test.vector[0m
        [1;94m   | [0m
    "#]]
    .assert_eq(&datamodel.to_string());
}

struct TestExtensions {
    types: Vec<(String, String, usize, Option<Vec<String>>)>,
}

impl ExtensionTypes for TestExtensions {
    fn get_by_prisma_name(&self, name: &str) -> Option<ExtensionTypeId> {
        self.types
            .iter()
            .position(|(t, _, _, _)| t == name)
            .map(ExtensionTypeId::from)
    }

    fn get_by_db_name_and_modifiers(&self, name: &str, modifiers: Option<&[String]>) -> Option<ExtensionTypeEntry<'_>> {
        self.types
            .iter()
            .enumerate()
            .find(|(_, (_, db_name, _, db_type_modifiers))| {
                db_name == name && db_type_modifiers.as_deref() == modifiers
            })
            .or_else(|| {
                self.types
                    .iter()
                    .enumerate()
                    .find(|(_, (_, db_name, _, db_type_modifiers))| db_name == name && db_type_modifiers.is_none())
            })
            .map(
                |(i, (prisma_name, db_name, number_of_args, expected_db_type_modifiers))| ExtensionTypeEntry {
                    id: ExtensionTypeId::from(i),
                    prisma_name: prisma_name.as_str(),
                    db_namespace: None,
                    db_name: db_name.as_str(),
                    number_of_args: *number_of_args,
                    db_type_modifiers: expected_db_type_modifiers.as_deref(),
                },
            )
    }

    fn enumerate(&self) -> Box<dyn Iterator<Item = psl::parser_database::ExtensionTypeEntry<'_>> + '_> {
        Box::new(self.types.iter().enumerate().map(
            |(i, (prisma_name, db_name, number_of_args, expected_db_type_modifiers))| ExtensionTypeEntry {
                id: ExtensionTypeId::from(i),
                prisma_name: prisma_name.as_str(),
                db_namespace: None,
                db_name: db_name.as_str(),
                number_of_args: *number_of_args,
                db_type_modifiers: expected_db_type_modifiers.as_deref(),
            },
        ))
    }
}
