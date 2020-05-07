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
        }"#
    );

    let mut dml = datamodel::parse_datamodel(input).unwrap();
    let cat = dml.models_mut().find(|m| m.name == "Category").unwrap();
    let name = cat.fields.iter_mut().find(|f| f.name == "name").unwrap();
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
        }"#
    );

    let dml = datamodel::parse_datamodel(input).unwrap();
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
        }"#
    );

    let mut dml = datamodel::parse_datamodel(input).unwrap();
    let cat = dml.models_mut().find(|m| m.name == "Category").unwrap();
    let name = cat.fields.iter_mut().find(|f| f.name == "name").unwrap();
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
        }"#
    );

    let dml = datamodel::parse_datamodel(input).unwrap();
    let rendered = datamodel::render_datamodel_to_string(&dml).unwrap();

    assert_eq!(input, rendered);
}
