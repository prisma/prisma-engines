use indoc::indoc;
use pretty_assertions::assert_eq;

#[test]
fn strings_with_quotes_are_unescaped() {
    let input = indoc!(
        r#"
        model Category {
          id   String @id
          name String @default("a \" b\"c d")
        }"#
    );

    let mut dml = datamodel::parse_datamodel(input).unwrap();
    let cat = dml.models_mut().find(|m| m.name == "Category").unwrap();
    let name = cat.scalar_fields().find(|f| f.name == "name").unwrap();

    assert_eq!(
        name.default_value
            .as_ref()
            .unwrap()
            .get()
            .unwrap()
            .into_string()
            .unwrap(),
        "a \" b\"c d"
    );
}

#[test]
fn strings_with_newlines_are_unescpaed() {
    let input = indoc!(
        r#"
        model Category {
          id   String @id
          name String @default("Jean\nClaude\nVan\nDamme")
        }"#
    );

    let mut dml = datamodel::parse_datamodel(input).unwrap();
    let cat = dml.models_mut().find(|m| m.name == "Category").unwrap();
    let name = cat.scalar_fields().find(|f| f.name == "name").unwrap();

    assert_eq!(
        name.default_value
            .as_ref()
            .unwrap()
            .get()
            .unwrap()
            .into_string()
            .unwrap(),
        "Jean\nClaude\nVan\nDamme"
    );
}
