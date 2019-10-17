#![allow(non_snake_case)]
mod test_harness;

use datamodel::ast::{parser, FieldArity, SchemaAst};
use migration_connector::steps::*;
use migration_core::migration::datamodel_migration_steps_inferrer::*;
use pretty_assertions::assert_eq;

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

    assert_eq!(infer(&dm1, &dm1), &[]);

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

    assert_eq!(infer(&dm1, &dm1), vec![]);

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

#[test]
fn infer_UpdateEnum() {
    let dm1 = parse(
        r#"
            enum Color {
                RED
                GREEN
                BLUE
            }
        "#,
    );

    assert_eq!(infer(&dm1, &dm1), &[]);

    let dm2 = parse(
        r#"

            enum Color {
                GREEN
                BEIGE
                BLUE
            }
        "#,
    );

    let steps = infer(&dm1, &dm2);
    let expected = vec![MigrationStep::UpdateEnum(UpdateEnum {
        name: "Color".to_owned(),
        created_values: vec!["BEIGE".to_owned()],
        deleted_values: vec!["RED".to_owned()],
        new_name: None,
    })];

    assert_eq!(steps, expected);
}

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

    let expected = &[MigrationStep::CreateField(CreateField {
        model: "User".into(),
        name: "invitedBy".into(),
        tpe: "User".to_owned(),
        arity: FieldArity::Optional,
        default: None,
        db_name: None,
    })];

    assert_eq!(steps, expected);
}

#[test]
fn infer_CreateDirective_on_field() {
    let dm1 = parse(
        r##"
        model User {
            id Int @id
            name String
        }
    "##,
    );

    let dm2 = parse(
        r##"
        model User {
            id Int @id
            name String @map("handle")
        }
    "##,
    );

    let steps = infer(&dm1, &dm2);

    let locator = DirectiveLocator {
        name: "map".to_owned(),
        location: DirectiveLocation::Field {
            model: "User".to_owned(),
            field: "name".to_owned(),
        },
    };

    let expected = &[
        MigrationStep::CreateDirective(CreateDirective {
            locator: locator.clone(),
        }),
        MigrationStep::CreateDirectiveArgument(CreateDirectiveArgument {
            directive_location: locator,
            argument_name: "".to_owned(),
            argument_value: MigrationExpression("\"handle\"".to_owned()),
        }),
    ];

    assert_eq!(steps, expected);
}

#[test]
fn infer_CreateDirective_on_model() {
    let dm1 = parse(
        r##"
        model User {
            id Int @id
            name String
        }
    "##,
    );

    let dm2 = parse(
        r##"
        model User {
            id Int @id
            name String

            @@map("customer")
        }
    "##,
    );

    let steps = infer(&dm1, &dm2);

    let locator = DirectiveLocator {
        name: "map".to_owned(),
        location: DirectiveLocation::Model {
            model: "User".to_owned(),
        },
    };

    let expected = &[
        MigrationStep::CreateDirective(CreateDirective {
            locator: locator.clone(),
        }),
        MigrationStep::CreateDirectiveArgument(CreateDirectiveArgument {
            directive_location: locator,
            argument_name: "".to_owned(),
            argument_value: MigrationExpression("\"customer\"".to_owned()),
        }),
    ];

    assert_eq!(steps, expected);
}

#[test]
fn infer_CreateDirective_on_enum() {
    let dm1 = parse(
        r##"
            enum Color {
                RED
                GREEN
                BLUE
            }
        "##,
    );

    let dm2 = parse(
        r##"
            enum Color {
                RED
                GREEN
                BLUE

                @@map("colour")
            }
        "##,
    );

    let steps = infer(&dm1, &dm2);

    let locator = DirectiveLocator {
        name: "map".to_owned(),
        location: DirectiveLocation::Enum {
            r#enum: "Color".to_owned(),
        },
    };

    let expected = &[
        MigrationStep::CreateDirective(CreateDirective {
            locator: locator.clone(),
        }),
        MigrationStep::CreateDirectiveArgument(CreateDirectiveArgument {
            directive_location: locator,
            argument_name: "".to_owned(),
            argument_value: MigrationExpression("\"colour\"".to_owned()),
        }),
    ];

    assert_eq!(steps, expected);
}

#[test]
fn infer_DeleteDirective_on_field() {
    let dm1 = parse(
        r##"
        model User {
            id Int @id
            name String @map("handle")
        }
    "##,
    );

    let dm2 = parse(
        r##"
        model User {
            id Int @id
            name String
        }
    "##,
    );

    let steps = infer(&dm1, &dm2);

    let locator = DirectiveLocator {
        name: "map".to_owned(),
        location: DirectiveLocation::Field {
            model: "User".to_owned(),
            field: "name".to_owned(),
        },
    };

    let expected = &[MigrationStep::DeleteDirective(DeleteDirective { locator })];

    assert_eq!(steps, expected);
}

#[test]
fn infer_DeleteDirective_on_model() {
    let dm1 = parse(
        r##"
        model User {
            id Int @id
            name String

            @@map("customer")
        }
    "##,
    );

    let dm2 = parse(
        r##"
        model User {
            id Int @id
            name String
        }
    "##,
    );

    let steps = infer(&dm1, &dm2);

    let locator = DirectiveLocator {
        name: "map".to_owned(),
        location: DirectiveLocation::Model {
            model: "User".to_owned(),
        },
    };

    let expected = &[MigrationStep::DeleteDirective(DeleteDirective {
        locator: locator.clone(),
    })];

    assert_eq!(steps, expected);
}

#[test]
fn infer_DeleteDirective_on_enum() {
    let dm1 = parse(
        r##"
            enum Color {
                RED
                GREEN
                BLUE

                @@map("colour")
            }

        "##,
    );

    let dm2 = parse(
        r##"
            enum Color {
                RED
                GREEN
                BLUE
            }
        "##,
    );

    let steps = infer(&dm1, &dm2);

    let locator = DirectiveLocator {
        name: "map".to_owned(),
        location: DirectiveLocation::Enum {
            r#enum: "Color".to_owned(),
        },
    };

    let expected = &[MigrationStep::DeleteDirective(DeleteDirective { locator })];

    assert_eq!(steps, expected);
}

fn infer(dm1: &SchemaAst, dm2: &SchemaAst) -> Vec<MigrationStep> {
    let inferrer = DataModelMigrationStepsInferrerImplWrapper {};
    inferrer.infer(&dm1, &dm2)
}

fn parse(input: &str) -> SchemaAst {
    parser::parse(input).unwrap()
}
