#![allow(non_snake_case)]

use datamodel::ast::FieldArity;
use migration_connector::steps::*;

#[test]
fn full_CreateModel_must_work() {
    let json = r#"{"stepType":"CreateModel","model":"Blog"}"#;
    let expected_struct = MigrationStep::CreateModel(CreateModel {
        model: "Blog".to_string(),
    });
    assert_symmetric_serde(json, expected_struct);
}

#[test]
fn minimal_UpdateModel_must_work() {
    let json = r#"{"stepType":"UpdateModel","model":"Blog"}"#;
    let expected_struct = MigrationStep::UpdateModel(UpdateModel {
        model: "Blog".to_string(),
        new_name: None,
    });
    assert_symmetric_serde(json, expected_struct);
}

#[test]
fn full_UpdateModel_must_work() {
    let json = r#"{"stepType":"UpdateModel","model":"Blog","newName":"MyBlog"}"#;
    let expected_struct = MigrationStep::UpdateModel(UpdateModel {
        model: "Blog".to_string(),
        new_name: Some("MyBlog".to_string()),
    });
    assert_symmetric_serde(json, expected_struct);
}

#[test]
fn DeleteModel_must_work() {
    let json = r#"{"stepType":"DeleteModel","model":"Blog"}"#;
    let expected_struct = MigrationStep::DeleteModel(DeleteModel {
        model: "Blog".to_string(),
    });
    assert_symmetric_serde(json, expected_struct);
}

#[test]
fn minimal_CreateField_must_work() {
    let json = r#"
            {
                "stepType":"CreateField",
                "model":"Blog",
                "field":"title",
                "type":"String",
                "arity":"required"
            }
        "#;
    let expected_struct = MigrationStep::CreateField(CreateField {
        model: "Blog".to_string(),
        field: "title".to_string(),
        tpe: "String".to_owned(),
        arity: FieldArity::Required,
    });
    assert_symmetric_serde(json, expected_struct);
}

#[test]
fn full_CreateField_must_work() {
    let json = r#"{
            "stepType":"CreateField",
            "model": "Blog",
            "field": "title",
            "type": "String",
            "arity": "optional"
        }"#;
    let expected_struct = MigrationStep::CreateField(CreateField {
        model: "Blog".to_string(),
        field: "title".to_string(),
        tpe: "String".to_owned(),
        arity: FieldArity::Optional,
    });

    assert_symmetric_serde(json, expected_struct);
}

#[test]
fn minimal_UpdateField_must_work() {
    let json = r#"{"stepType":"UpdateField","model":"Blog","field":"title"}"#;
    let expected_struct = MigrationStep::UpdateField(UpdateField {
        model: "Blog".to_string(),
        field: "title".to_string(),
        new_name: None,
        tpe: None,
        arity: None,
    });
    assert_symmetric_serde(json, expected_struct);
}

#[test]
fn full_UpdateField_must_work() {
    let json = r#"
        {
            "stepType": "UpdateField",
            "model": "Blog",
            "field": "title",
            "newName": "MyBlog",
            "type": "String",
            "arity": "optional"
        }
    "#;
    let expected_struct = MigrationStep::UpdateField(UpdateField {
        model: "Blog".to_string(),
        field: "title".to_string(),
        new_name: Some("MyBlog".to_string()),
        tpe: Some("String".to_owned()),
        arity: Some(FieldArity::Optional),
    });
    assert_symmetric_serde(json, expected_struct);
}

#[test]
fn DeleteField_must_work() {
    let json = r#"{"stepType":"DeleteField","model":"Blog","field":"title"}"#;
    let expected_struct = MigrationStep::DeleteField(DeleteField {
        model: "Blog".to_string(),
        field: "title".to_string(),
    });
    assert_symmetric_serde(json, expected_struct);
}

#[test]
fn CreateEnum_must_work() {
    let json = r#"
        {
            "stepType": "CreateEnum",
            "enum": "BlogCategory",
            "values": ["Politics","Tech"]
        }
    "#;
    let expected_struct = MigrationStep::CreateEnum(CreateEnum {
        r#enum: "BlogCategory".to_string(),
        values: vec!["Politics".to_string(), "Tech".to_string()],
    });
    assert_symmetric_serde(json, expected_struct);
}

#[test]
fn minimal_UpdateEnum_must_work() {
    let json = r#"
        {
            "stepType": "UpdateEnum",
            "enum": "BlogCategory"
        }
    "#;
    let expected_struct = MigrationStep::UpdateEnum(UpdateEnum {
        r#enum: "BlogCategory".to_string(),
        new_name: None,
        created_values: vec![],
        deleted_values: vec![],
    });
    assert_symmetric_serde(json, expected_struct);
}

#[test]
fn full_Update_Enum_must_work() {
    let json = r#"
        {
            "stepType": "UpdateEnum",
            "enum": "BlogCategory",
            "newName": "MyBlogCategory",
            "createdValues": ["Tech"],
            "deletedValues": ["Nology"]
        }
    "#;
    let expected_struct = MigrationStep::UpdateEnum(UpdateEnum {
        r#enum: "BlogCategory".to_string(),
        new_name: Some("MyBlogCategory".to_string()),
        created_values: vec!["Tech".to_string()],
        deleted_values: vec!["Nology".to_string()],
    });
    assert_symmetric_serde(json, expected_struct);
}

#[test]
fn DeleteEnum_must_work() {
    let json = r#"{"stepType":"DeleteEnum","enum":"BlogCategory"}"#;
    let expected_struct = MigrationStep::DeleteEnum(DeleteEnum {
        r#enum: "BlogCategory".to_string(),
    });
    assert_symmetric_serde(json, expected_struct);
}

#[test]
fn CreateDirective_must_work() {
    let json = r#"
        {
            "stepType": "CreateDirective",
            "model": "Blog",
            "directive": "map"
        }
    "#;

    let expected_step = MigrationStep::CreateDirective(CreateDirective {
        locator: DirectiveLocation {
            directive: "map".to_owned(),
            arguments: None,
            location: DirectiveType::Model {
                model: "Blog".to_owned(),
            },
        },
    });

    assert_symmetric_serde(json, expected_step);
}

#[test]
fn minimal_DeleteDirective_must_work() {
    let json = r#"
        {
            "stepType": "DeleteDirective",
            "model": "Blog",
            "field": "title",
            "directive": "map"
        }
    "#;

    let expected_step = MigrationStep::DeleteDirective(DeleteDirective {
        locator: DirectiveLocation {
            directive: "map".to_owned(),
            location: DirectiveType::Field {
                model: "Blog".to_owned(),
                field: "title".to_owned(),
            },
            arguments: None,
        },
    });

    assert_symmetric_serde(json, expected_step);
}

#[test]
fn full_DeleteDirective_must_work() {
    let json = r#"
        {
            "stepType": "DeleteDirective",
            "model": "Blog",
            "directive": "unique",
            "arguments": [
                {
                    "name": "",
                    "value": "[name, age]"
                }
            ]
        }
    "#;

    let expected_step = MigrationStep::DeleteDirective(DeleteDirective {
        locator: DirectiveLocation {
            directive: "unique".to_owned(),
            location: DirectiveType::Model {
                model: "Blog".to_owned(),
            },
            arguments: Some(vec![Argument {
                name: "".to_owned(),
                value: MigrationExpression("[name, age]".to_owned()),
            }]),
        },
    });

    assert_symmetric_serde(json, expected_step);
}

#[test]
fn UpdateDirectiveArgument_must_work() {
    let json = r#"
        {
            "stepType": "UpdateDirectiveArgument",
            "enum": "CatMood",
            "directive": "map",
            "argument": "name",
            "newValue": "cat_mood"
        }
    "#;

    let expected_step = MigrationStep::UpdateDirectiveArgument(UpdateDirectiveArgument {
        directive_location: DirectiveLocation {
            directive: "map".to_owned(),
            location: DirectiveType::Enum {
                r#enum: "CatMood".to_owned(),
            },
            arguments: None,
        },
        argument: "name".to_owned(),
        new_value: MigrationExpression("cat_mood".to_owned()),
    });

    assert_symmetric_serde(json, expected_step);
}

#[test]
fn CreateDirectiveArgument_must_work() {
    let json = r#"
        {
            "stepType": "CreateDirectiveArgument",
            "enum": "CatMood",
            "directive": "map",
            "argument": "name",
            "value": "cat_mood"
        }
    "#;

    let expected_step = MigrationStep::CreateDirectiveArgument(CreateDirectiveArgument {
        directive_location: DirectiveLocation {
            arguments: None,
            directive: "map".to_owned(),
            location: DirectiveType::Enum {
                r#enum: "CatMood".to_owned(),
            },
        },
        argument: "name".to_owned(),
        value: MigrationExpression("cat_mood".to_owned()),
    });

    assert_symmetric_serde(json, expected_step);
}

#[test]
fn DeleteDirectiveArgument_must_work() {
    let json = r#"
        {
            "stepType": "DeleteDirectiveArgument",
            "enum": "CatMood",
            "directive": "map",
            "argument": "name"
        }
    "#;

    let expected_step = MigrationStep::DeleteDirectiveArgument(DeleteDirectiveArgument {
        directive_location: DirectiveLocation {
            directive: "map".to_owned(),
            arguments: None,
            location: DirectiveType::Enum {
                r#enum: "CatMood".to_owned(),
            },
        },
        argument: "name".to_owned(),
    });

    assert_symmetric_serde(json, expected_step);
}

fn assert_symmetric_serde(json: &str, expected: MigrationStep) {
    let serde_value: serde_json::Value = serde_json::from_str(&json).expect("The provided input was invalid json.");
    let deserialized: MigrationStep = serde_json::from_str(&json).expect("Deserialization failed.");
    let serialized_again = serde_json::to_value(&deserialized).expect("Serialization failed");
    assert_eq!(
        deserialized, expected,
        "The provided json could not be serialized into the expected struct."
    );
    assert_eq!(
        serialized_again, serde_value,
        "Reserializing did not produce the original json input."
    );
}
