use indoc::indoc;
use schema_ast::ast::{IndentationType, NewlineType};

#[test]
fn default_spacing() {
    let input = indoc! {r#"
        model Category {
          id  Int    @id
          val String
        }
    "#};

    let db = datamodel::parse_schema_parserdb(input).unwrap().db;
    let model = db.walk_models().next().unwrap();

    assert_eq!(IndentationType::Spaces(2), model.indentation())
}

#[test]
fn four_space_indentation() {
    let input = indoc! {r#"
        model Category {
            id  Int    @id
            val String
        }
    "#};

    let db = datamodel::parse_schema_parserdb(input).unwrap().db;
    let model = db.walk_models().next().unwrap();

    assert_eq!(IndentationType::Spaces(4), model.indentation())
}

#[test]
fn tab_indentation() {
    let input = indoc! {r#"
        model Category {
        	id  Int    @id
        	val String
        }
    "#};

    let db = datamodel::parse_schema_parserdb(input).unwrap().db;
    let model = db.walk_models().next().unwrap();

    assert_eq!(IndentationType::Tabs, model.indentation())
}

#[test]
fn unix_newline() {
    let input = "model Category {\n  id Int @id\n}";

    let db = datamodel::parse_schema_parserdb(input).unwrap().db;
    let model = db.walk_models().next().unwrap();

    assert_eq!(NewlineType::Unix, model.newline())
}

#[test]
fn windows_newline() {
    let input = "model Category {\r\n  id Int @id\r\n}";

    let db = datamodel::parse_schema_parserdb(input).unwrap().db;
    let model = db.walk_models().next().unwrap();

    assert_eq!(NewlineType::Windows, model.newline())
}
