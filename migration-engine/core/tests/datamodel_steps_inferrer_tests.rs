#![allow(non_snake_case)]
#![allow(unused)]
mod test_harness;

use datamodel::ast::{parser, FieldArity, SchemaAst};
use migration_connector::steps::*;
use migration_core::migration::datamodel_migration_steps_inferrer::*;
use pretty_assertions::{assert_eq, assert_ne};

#[test]
fn infer_CreateModel_if_it_does_not_exist_yet() {
    let dm1 = SchemaAst::empty();
    let dm2 = parse(
        r#"
        model Test {
            id Int @id
        }
    "#,
    );

    let steps = infer(&dm1, &dm2);
    let expected = vec![
        MigrationStep::CreateModel(CreateModel {
            name: "Test".to_string(),
            db_name: None,
            embedded: false,
        }),
        MigrationStep::CreateField(CreateField {
            default: None,
            model: "Test".to_string(),
            name: "id".to_string(),
            tpe: "Int".to_owned(),
            arity: FieldArity::Required,
            db_name: None,
        }),
        MigrationStep::CreateDirective(CreateDirective {
            locator: DirectiveLocator {
                name: "id".to_owned(),
                location: DirectiveLocation::Field {
                    model: "Test".to_owned(),
                    field: "id".to_owned(),
                },
            },
        }),
    ];
    assert_eq!(steps, expected);
}

#[test]
fn infer_DeleteModel() {
    let dm1 = parse(
        r#"
        model Test {
            id String @id @default(cuid())
        }
    "#,
    );
    let dm2 = SchemaAst::empty();

    let steps = infer(&dm1, &dm2);
    let expected = vec![MigrationStep::DeleteModel(DeleteModel {
        name: "Test".to_string(),
    })];
    assert_eq!(steps, expected);
}

#[test]
fn infer_UpdateModel() {
    // TODO: add tests for other properties as well
    let dm1 = parse(
        r#"
        model Post {
            id String @id @default(cuid())
        }
    "#,
    );
    let dm2 = parse(
        r#"
        model Post{
            id String @id @default(cuid())

            @@embedded
        }
    "#,
    );

    let steps = infer(&dm1, &dm2);
    let expected = vec![MigrationStep::CreateDirective(CreateDirective {
        locator: DirectiveLocator {
            name: "embedded".to_owned(),
            location: DirectiveLocation::Model {
                model: "Post".to_owned(),
            },
        },
    })];
    assert_eq!(steps, expected);
}

#[test]
fn infer_CreateField_if_it_does_not_exist_yet() {
    let dm1 = parse(
        r#"
        model Test {
            id String @id @default(cuid())
        }
    "#,
    );
    let dm2 = parse(
        r#"
        model Test {
            id String @id @default(cuid())
            field Int?
        }
    "#,
    );

    let steps = infer(&dm1, &dm2);
    let expected = vec![MigrationStep::CreateField(CreateField {
        model: "Test".to_string(),
        name: "field".to_string(),
        tpe: "Int".to_owned(),
        arity: FieldArity::Optional,
        db_name: None,
        default: None,
    })];
    assert_eq!(steps, expected);
}

#[test]
fn infer_CreateField_if_relation_field_does_not_exist_yet() {
    let dm1 = parse(
        r#"
        model Blog {
            id String @id @default(cuid())
        }
        model Post {
            id String @id @default(cuid())
        }
    "#,
    );
    let dm2 = parse(
        r#"
        model Blog {
            id String @id @default(cuid())
            posts Post[]
        }
        model Post {
            id String @id @default(cuid())
            blog Blog?
        }
    "#,
    );

    let steps = infer(&dm1, &dm2);
    let expected = vec![
        MigrationStep::CreateField(CreateField {
            model: "Blog".to_string(),
            name: "posts".to_string(),
            tpe: "Post".to_owned(),
            arity: FieldArity::List,
            db_name: None,
            default: None,
        }),
        MigrationStep::CreateField(CreateField {
            model: "Post".to_string(),
            name: "blog".to_string(),
            tpe: "Blog".to_owned(),
            arity: FieldArity::Optional,
            db_name: None,
            default: None,
        }),
    ];
    assert_eq!(steps, expected);
}

#[test]
fn infer_DeleteField() {
    let dm1 = parse(
        r#"
        model Test {
            id String @id @default(cuid())
            field Int?
        }
    "#,
    );
    let dm2 = parse(
        r#"
        model Test {
            id String @id @default(cuid())
        }
    "#,
    );

    let steps = infer(&dm1, &dm2);
    let expected = vec![MigrationStep::DeleteField(DeleteField {
        model: "Test".to_string(),
        name: "field".to_string(),
    })];
    assert_eq!(steps, expected);
}

#[test]
fn infer_UpdateField_simple() {
    let dm1 = parse(
        r#"
        model Test {
            id String @id @default(cuid())
            field Int?
        }
    "#,
    );
    let dm2 = parse(
        r#"
        model Test {
            id String @id @default(cuid())
            field Boolean @default(false) @unique
        }
    "#,
    );

    let steps = infer(&dm1, &dm2);
    let expected = vec![
        MigrationStep::UpdateField(UpdateField {
            model: "Test".to_string(),
            name: "field".to_string(),
            new_name: None,
            tpe: Some("Boolean".to_owned()),
            arity: Some(FieldArity::Required),
            default: Some(Some(MigrationExpression("false".to_owned()))),
        }),
        MigrationStep::CreateDirective(CreateDirective {
            locator: DirectiveLocator {
                name: "default".to_owned(),
                location: DirectiveLocation::Field {
                    model: "Test".to_owned(),
                    field: "field".to_owned(),
                },
            },
        }),
        MigrationStep::CreateDirectiveArgument(CreateDirectiveArgument {
            directive_location: DirectiveLocator {
                name: "default".to_owned(),
                location: DirectiveLocation::Field {
                    model: "Test".to_owned(),
                    field: "field".to_owned(),
                },
            },
            argument_name: "".to_owned(),
            argument_value: MigrationExpression("false".to_owned()),
        }),
        MigrationStep::CreateDirective(CreateDirective {
            locator: DirectiveLocator {
                name: "unique".to_owned(),
                location: DirectiveLocation::Field {
                    model: "Test".to_owned(),
                    field: "field".to_owned(),
                },
            },
        }),
    ];
    assert_eq!(steps, expected);
}

#[test]
fn infer_CreateEnum() {
    let dm1 = SchemaAst::empty();
    let dm2 = parse(
        r#"
        enum Test {
            A
            B
        }
    "#,
    );

    let steps = infer(&dm1, &dm2);
    let expected = vec![MigrationStep::CreateEnum(CreateEnum {
        name: "Test".to_string(),
        values: vec!["A".to_string(), "B".to_string()],
    })];
    assert_eq!(steps, expected);
}

#[test]
fn infer_DeleteEnum() {
    let dm1 = parse(
        r#"
        enum Test {
            A
            B
        }
    "#,
    );
    let dm2 = SchemaAst::empty();

    let steps = infer(&dm1, &dm2);
    let expected = vec![MigrationStep::DeleteEnum(DeleteEnum {
        name: "Test".to_string(),
    })];
    assert_eq!(steps, expected);
}

// #[test]
// fn infer_UpdateIndex() {
//     let dm1 = parse(
//         r#"
//         model Dog {
//             id Int @id
//             age Int
//             name String

//             @@unique([age, name], name: "customDogIndex")
//         }
//         "#,
//     );

//     let dm2 = parse(
//         r#"
//         model Dog {
//             id Int @id
//             age Int
//             name String

//             @@unique([age, name], name: "customDogIndex2")
//         }
//         "#,
//     );

//     let steps = infer(&dm1, &dm2);
//     let expected = vec![MigrationStep::UpdateIndex(UpdateIndex {
//         model: "Dog".into(),
//         name: Some("customDogIndex2".into()),
//         tpe: IndexType::Unique,
//         fields: vec!["age".into(), "name".into()],
//     })];

//     assert_eq!(steps, expected);
// }

#[test]
fn infer_CreateField_on_self_relation() {
    let dm1 = parse(
        r#"
            model User {
                id Int @id
            }
        "#,
    );

    let dm2 = parse(
        r#"
            model User {
                id Int @id
                invitedBy User?
            }
        "#,
    );

    let steps = infer(&dm1, &dm2);

    let expected = vec![MigrationStep::CreateField(CreateField {
        model: "User".into(),
        name: "invitedBy".into(),
        tpe: "User".to_owned(),
        arity: FieldArity::Optional,
        default: None,
        db_name: None,
    })];
}

fn infer(dm1: &SchemaAst, dm2: &SchemaAst) -> Vec<MigrationStep> {
    let inferrer = DataModelMigrationStepsInferrerImplWrapper {};
    inferrer.infer(&dm1, &dm2)
}

fn parse(input: &str) -> SchemaAst {
    parser::parse(input).unwrap()
}
