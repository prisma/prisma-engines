use datamodel::DefaultValue;
use indoc::indoc;
use pretty_assertions::assert_eq;
use prisma_value::PrismaValue;

#[test]
fn strings_with_quotes_render_as_escaped_literals() {
    let input = indoc!(
        r#"
        model Category {
          id   String @id
          name String
        }"#
    );

    let expected = indoc!(
        r#"
        model Category {
          id   String @id
          name String @default("a \" b\"c d")
        }
        "#
    );

    let mut dml = datamodel::parse_datamodel(input).unwrap().subject;
    let cat = dml.models_mut().find(|m| m.name == "Category").unwrap();
    let name = cat.scalar_fields_mut().find(|f| f.name == "name").unwrap();
    name.default_value = Some(DefaultValue::Single(PrismaValue::String("a \" b\"c d".into())));

    let rendered = datamodel::render_datamodel_to_string(&dml).unwrap();

    assert_eq!(rendered, expected);
}

#[test]
fn strings_with_quotes_roundtrip() {
    let input = indoc!(
        r#"
        model Category {
          id   String @id
          name String @default("a \" b\"c d")
        }
        "#
    );

    let dml = datamodel::parse_datamodel(input).unwrap().subject;
    let rendered = datamodel::render_datamodel_to_string(&dml).unwrap();

    assert_eq!(input, rendered);
}

#[test]
fn strings_with_newlines_render_as_escaped_literals() {
    let input = indoc!(
        r#"
        model Category {
          id   String @id
          name String
        }"#
    );

    let expected = indoc!(
        r#"
        model Category {
          id   String @id
          name String @default("Jean\nClaude\nVan\nDamme")
        }
        "#
    );

    let mut dml = datamodel::parse_datamodel(input).unwrap().subject;
    let cat = dml.models_mut().find(|m| m.name == "Category").unwrap();
    let name = cat.scalar_fields_mut().find(|f| f.name == "name").unwrap();
    name.default_value = Some(DefaultValue::Single(PrismaValue::String(
        "Jean\nClaude\nVan\nDamme".into(),
    )));

    let rendered = datamodel::render_datamodel_to_string(&dml).unwrap();

    assert_eq!(rendered, expected);
}

#[test]
fn strings_with_newlines_roundtrip() {
    let input = indoc!(
        r#"
        model Category {
          id   String @id
          name String @default("Jean\nClaude\nVan\nDamme")
        }
        "#
    );

    let dml = datamodel::parse_datamodel(input).unwrap().subject;
    let rendered = datamodel::render_datamodel_to_string(&dml).unwrap();

    assert_eq!(input, rendered);
}

#[test]
fn strings_with_backslashes_roundtrip() {
    let input = indoc!(
        r#"
        model Category {
          id   String @id
          name String @default("xyz\\Datasource\\Model")
        }
        "#
    );

    let dml = datamodel::parse_datamodel(input).unwrap().subject;
    let rendered = datamodel::render_datamodel_to_string(&dml).unwrap();

    assert_eq!(input, rendered);
}

#[test]
fn strings_with_multiple_escaped_characters_roundtrip() {
    let dm = indoc!(
        r#"
        model FilmQuote {
          id             Int    @id
          favouriteQuote String @default("\"That's a lot of fish\"\n - Godzilla (1998)")
        }
        "#
    );

    let dml = datamodel::parse_datamodel(dm).unwrap().subject;
    let rendered = datamodel::render_datamodel_to_string(&dml).unwrap();

    assert_eq!(dm, rendered);
}

#[test]
fn internal_escaped_values_are_rendered_correctly() {
    let dm = indoc!(
        r#"
        model FilmQuote {
          id             Int    @id
        }"#
    );

    let expected_dm = indoc!(
        r#"
        model FilmQuote {
          id Int @id @default("xyz\\Datasource\\Model")
        }
        "#
    );

    let mut dml = datamodel::parse_datamodel(dm).unwrap().subject;

    let model = dml.models_mut().find(|m| m.name == "FilmQuote").unwrap();
    let field = model.scalar_fields_mut().find(|f| f.name == "id").unwrap();
    field.default_value = Some(DefaultValue::Single(PrismaValue::String(
        "xyz\\Datasource\\Model".to_string(),
    )));

    let rendered = datamodel::render_datamodel_to_string(&dml).unwrap();

    assert_eq!(expected_dm, rendered);
}
