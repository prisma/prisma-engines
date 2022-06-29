/*
use expect_test::expect;
use indoc::indoc;

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

    //expect![["  "]].assert_eq(&model.ast_model().indentation_type.to_string());
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

    //expect![["    "]].assert_eq(&model.ast_model().indentation_type.to_string());
}
*/
