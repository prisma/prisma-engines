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
    let expected = &[
        MigrationStep::CreateModel(CreateModel {
            model: "Test".to_string(),
        }),
        MigrationStep::CreateField(CreateField {
            model: "Test".to_string(),
            field: "id".to_string(),
            tpe: "Int".to_owned(),
            arity: FieldArity::Required,
        }),
        MigrationStep::CreateArgumentContainer(CreateArgumentContainer {
            location: ArgumentLocation {
                arguments: None,
                argument_container: "id".to_owned(),
                argument_type: ArgumentType::FieldDirective {
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
    let expected = &[MigrationStep::DeleteModel(DeleteModel {
        model: "Test".to_string(),
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

            @@unique([id])
            @@index([id])
        }
    "#,
    );

    assert_eq!(infer(&dm1, &dm1), &[]);

    let dm2 = parse(
        r#"
        model Post{
            id String @id @default(cuid())

            @@embedded
            @@unique([id])
            @@index([id])
        }
    "#,
    );

    let steps = infer(&dm1, &dm2);
    let expected = &[MigrationStep::CreateArgumentContainer(CreateArgumentContainer {
        location: ArgumentLocation {
            arguments: None,
            argument_container: "embedded".to_owned(),
            argument_type: ArgumentType::ModelDirective {
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
    let expected = &[MigrationStep::CreateField(CreateField {
        model: "Test".to_string(),
        field: "field".to_string(),
        tpe: "Int".to_owned(),
        arity: FieldArity::Optional,
    })];
    assert_eq!(steps, expected);
}

#[test]
fn infer_CreateField_with_default() {
    let dm1 = parse(
        r#"
            model Test {
                id Int @id
            }
        "#,
    );
    let dm2 = parse(
        r#"
            model Test {
                id Int @id
                isReady Boolean @default(false)
            }
        "#,
    );

    let steps = infer(&dm1, &dm2);

    let expected = &[
        MigrationStep::CreateField(CreateField {
            model: "Test".to_owned(),
            field: "isReady".to_owned(),
            tpe: "Boolean".to_owned(),
            arity: FieldArity::Required,
        }),
        MigrationStep::CreateArgumentContainer(CreateArgumentContainer {
            location: ArgumentLocation {
                arguments: None,
                argument_type: ArgumentType::FieldDirective {
                    model: "Test".to_owned(),
                    field: "isReady".to_owned(),
                },
                argument_container: "default".to_owned(),
            },
        }),
        MigrationStep::CreateArgument(CreateArgument {
            location: ArgumentLocation {
                arguments: None,
                argument_type: ArgumentType::FieldDirective {
                    model: "Test".to_owned(),
                    field: "isReady".to_owned(),
                },
                argument_container: "default".to_owned(),
            },
            argument: "".to_owned(),
            value: MigrationExpression("false".to_owned()),
        }),
    ];

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
            field: "posts".to_string(),
            tpe: "Post".to_owned(),
            arity: FieldArity::List,
        }),
        MigrationStep::CreateField(CreateField {
            model: "Post".to_string(),
            field: "blog".to_string(),
            tpe: "Blog".to_owned(),
            arity: FieldArity::Optional,
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
        field: "field".to_string(),
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
    let expected = &[
        MigrationStep::UpdateField(UpdateField {
            model: "Test".to_string(),
            field: "field".to_string(),
            new_name: None,
            tpe: Some("Boolean".to_owned()),
            arity: Some(FieldArity::Required),
        }),
        MigrationStep::CreateArgumentContainer(CreateArgumentContainer {
            location: ArgumentLocation {
                argument_container: "default".to_owned(),
                arguments: None,
                argument_type: ArgumentType::FieldDirective {
                    model: "Test".to_owned(),
                    field: "field".to_owned(),
                },
            },
        }),
        MigrationStep::CreateArgument(CreateArgument {
            location: ArgumentLocation {
                arguments: None,
                argument_container: "default".to_owned(),
                argument_type: ArgumentType::FieldDirective {
                    model: "Test".to_owned(),
                    field: "field".to_owned(),
                },
            },
            argument: "".to_owned(),
            value: MigrationExpression("false".to_owned()),
        }),
        MigrationStep::CreateArgumentContainer(CreateArgumentContainer {
            location: ArgumentLocation {
                arguments: None,
                argument_container: "unique".to_owned(),
                argument_type: ArgumentType::FieldDirective {
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
        r#enum: "Test".to_string(),
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
        r#enum: "Test".to_string(),
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
        r#enum: "Color".to_owned(),
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
        field: "invitedBy".into(),
        tpe: "User".to_owned(),
        arity: FieldArity::Optional,
    })];

    assert_eq!(steps, expected);
}

#[test]
fn infer_CreateArgumentContainer_on_field() {
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

    let locator = ArgumentLocation {
        argument_container: "map".to_owned(),
        arguments: None,
        argument_type: ArgumentType::FieldDirective {
            model: "User".to_owned(),
            field: "name".to_owned(),
        },
    };

    let expected = &[
        MigrationStep::CreateArgumentContainer(CreateArgumentContainer {
            location: locator.clone(),
        }),
        MigrationStep::CreateArgument(CreateArgument {
            location: locator,
            argument: "".to_owned(),
            value: MigrationExpression("\"handle\"".to_owned()),
        }),
    ];

    assert_eq!(steps, expected);
}

#[test]
fn infer_CreateArgumentContainer_on_model() {
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

    let locator = ArgumentLocation {
        argument_container: "map".to_owned(),
        arguments: None,
        argument_type: ArgumentType::ModelDirective {
            model: "User".to_owned(),
        },
    };

    let expected = &[
        MigrationStep::CreateArgumentContainer(CreateArgumentContainer {
            location: locator.clone(),
        }),
        MigrationStep::CreateArgument(CreateArgument {
            location: locator,
            argument: "".to_owned(),
            value: MigrationExpression("\"customer\"".to_owned()),
        }),
    ];

    assert_eq!(steps, expected);
}

#[test]
fn infer_CreateArgumentContainer_on_model_repeated_directive() {
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

            @@unique([name])
        }
    "##,
    );

    let steps = infer(&dm1, &dm2);

    let locator = ArgumentLocation {
        argument_container: "unique".to_owned(),
        arguments: Some(vec![Argument {
            name: "".to_owned(),
            value: MigrationExpression("[name]".to_owned()),
        }]),
        argument_type: ArgumentType::ModelDirective {
            model: "User".to_owned(),
        },
    };

    let expected = &[MigrationStep::CreateArgumentContainer(CreateArgumentContainer {
        location: locator,
    })];

    assert_eq!(steps, expected);
}

#[test]
fn infer_CreateArgumentContainer_on_enum() {
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

    let locator = ArgumentLocation {
        argument_container: "map".to_owned(),
        arguments: None,
        argument_type: ArgumentType::EnumDirective {
            r#enum: "Color".to_owned(),
        },
    };

    let expected = &[
        MigrationStep::CreateArgumentContainer(CreateArgumentContainer {
            location: locator.clone(),
        }),
        MigrationStep::CreateArgument(CreateArgument {
            location: locator,
            argument: "".to_owned(),
            value: MigrationExpression("\"colour\"".to_owned()),
        }),
    ];

    assert_eq!(steps, expected);
}

#[test]
fn infer_CreateArgumentContainer_on_type_alias() {
    let dm1 = parse(r#"type BlogPost = String @default("a")"#);
    let dm2 = parse(r#"type BlogPost = String @customized @default("a")"#);

    let steps = infer(&dm1, &dm2);

    let locator = ArgumentLocation {
        argument_container: "customized".to_owned(),
        arguments: None,
        argument_type: ArgumentType::TypeAlias {
            type_alias: "BlogPost".to_owned(),
        },
    };

    let expected = &[MigrationStep::CreateArgumentContainer(CreateArgumentContainer {
        location: locator,
    })];

    assert_eq!(steps, expected);
}

#[test]
fn infer_DeleteArgumentContainer_on_field() {
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

    let locator = ArgumentLocation {
        argument_container: "map".to_owned(),
        arguments: None,
        argument_type: ArgumentType::FieldDirective {
            model: "User".to_owned(),
            field: "name".to_owned(),
        },
    };

    let expected = &[MigrationStep::DeleteArgumentContainer(DeleteArgumentContainer {
        location: locator,
    })];

    assert_eq!(steps, expected);
}

#[test]
fn infer_DeleteArgumentContainer_on_model() {
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

    let locator = ArgumentLocation {
        argument_container: "map".to_owned(),
        arguments: None,
        argument_type: ArgumentType::ModelDirective {
            model: "User".to_owned(),
        },
    };

    let expected = &[MigrationStep::DeleteArgumentContainer(DeleteArgumentContainer {
        location: locator,
    })];

    assert_eq!(steps, expected);
}

#[test]
fn infer_DeleteArgumentContainer_on_model_repeated_directive() {
    let dm1 = parse(
        r##"
        model User {
            id Int @id
            name String

            @@unique([name])
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

    let locator = ArgumentLocation {
        argument_container: "unique".to_owned(),
        arguments: Some(vec![Argument {
            name: "".to_owned(),
            value: MigrationExpression("[name]".to_owned()),
        }]),
        argument_type: ArgumentType::ModelDirective {
            model: "User".to_owned(),
        },
    };

    let expected = &[MigrationStep::DeleteArgumentContainer(DeleteArgumentContainer {
        location: locator,
    })];

    assert_eq!(steps, expected);
}

#[test]
fn infer_DeleteArgumentContainer_on_enum() {
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

    assert_eq!(infer(&dm1, &dm1), &[]);

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

    let locator = ArgumentLocation {
        argument_container: "map".to_owned(),
        arguments: None,
        argument_type: ArgumentType::EnumDirective {
            r#enum: "Color".to_owned(),
        },
    };

    let expected = &[MigrationStep::DeleteArgumentContainer(DeleteArgumentContainer {
        location: locator,
    })];

    assert_eq!(steps, expected);
}

#[test]
fn infer_DeleteArgumentContainer_on_type_alias() {
    let dm1 = parse(r#"type BlogPost = String @default("chimken")"#);
    let dm2 = parse(r#"type BlogPost = String"#);

    let steps = infer(&dm1, &dm2);

    let locator = ArgumentLocation {
        argument_container: "default".to_owned(),
        arguments: None,
        argument_type: ArgumentType::TypeAlias {
            type_alias: "BlogPost".to_owned(),
        },
    };

    let expected = &[MigrationStep::DeleteArgumentContainer(DeleteArgumentContainer {
        location: locator,
    })];

    assert_eq!(steps, expected);
}

#[test]
fn infer_CreateArgument_on_field() {
    let dm1 = parse(
        r##"
            model User {
                id Int @id
                name String @translate("German")
            }
        "##,
    );

    let dm2 = parse(
        r##"
            model User {
                id Int @id
                name String @translate("German", secondary: "ZH-CN", tertiary: "FR-BE")
            }
        "##,
    );

    let steps = infer(&dm1, &dm2);

    let locator = ArgumentLocation {
        argument_container: "translate".to_owned(),
        arguments: None,
        argument_type: ArgumentType::FieldDirective {
            model: "User".to_owned(),
            field: "name".to_owned(),
        },
    };

    let expected = &[
        MigrationStep::CreateArgument(CreateArgument {
            location: locator.clone(),
            argument: "secondary".to_owned(),
            value: MigrationExpression("\"ZH-CN\"".to_owned()),
        }),
        MigrationStep::CreateArgument(CreateArgument {
            location: locator,
            argument: "tertiary".to_owned(),
            value: MigrationExpression("\"FR-BE\"".to_owned()),
        }),
    ];

    assert_eq!(steps, expected);
}

#[test]
fn infer_CreateArgument_on_model() {
    let dm1 = parse(
        r##"
            model User {
                id Int @id
                name String

                @@randomDirective([name])
            }
        "##,
    );

    let dm2 = parse(
        r##"
            model User {
                id Int @id
                name String

                @@randomDirective([name], name: "usernameUniqueness")
            }
        "##,
    );

    let steps = infer(&dm1, &dm2);

    let locator = ArgumentLocation {
        arguments: None,
        argument_container: "randomDirective".to_owned(),
        argument_type: ArgumentType::ModelDirective {
            model: "User".to_owned(),
        },
    };

    let expected = &[MigrationStep::CreateArgument(CreateArgument {
        location: locator,
        argument: "name".to_owned(),
        value: MigrationExpression("\"usernameUniqueness\"".to_owned()),
    })];

    assert_eq!(steps, expected);
}

#[test]
fn infer_CreateArgument_on_enum() {
    let dm1 = parse(
        r##"
            enum EyeColor {
                BLUE
                GREEN
                BROWN

                @@random(one: "two")
            }
        "##,
    );

    assert_eq!(infer(&dm1, &dm1), &[]);

    let dm2 = parse(
        r##"
            enum EyeColor {
                BLUE
                GREEN
                BROWN

                @@random(one: "two", three: 4)
            }
        "##,
    );

    let steps = infer(&dm1, &dm2);

    let locator = ArgumentLocation {
        argument_container: "random".to_owned(),
        arguments: None,
        argument_type: ArgumentType::EnumDirective {
            r#enum: "EyeColor".to_owned(),
        },
    };

    let expected = &[MigrationStep::CreateArgument(CreateArgument {
        location: locator,
        argument: "three".to_owned(),
        value: MigrationExpression("4".to_owned()),
    })];

    assert_eq!(steps, expected);
}

#[test]
fn infer_CreateArgument_on_type_alias() {
    let dm1 = parse(r#"type BlogPost = String @customDirective(c: "d")"#);
    let dm2 = parse(r#"type BlogPost = String @customDirective(a: "b", c: "d")"#);

    let steps = infer(&dm1, &dm2);

    let locator = ArgumentLocation {
        argument_container: "customDirective".to_owned(),
        arguments: None,
        argument_type: ArgumentType::TypeAlias {
            type_alias: "BlogPost".to_owned(),
        },
    };

    let expected = &[MigrationStep::CreateArgument(CreateArgument {
        location: locator,
        argument: "a".to_owned(),
        value: MigrationExpression("\"b\"".to_owned()),
    })];

    assert_eq!(steps, expected);
}

#[test]
fn infer_DeleteArgument_on_field() {
    let dm1 = parse(
        r##"
            model User {
                id Int @id
                name String @translate("German", secondary: "ZH-CN", tertiary: "FR-BE")
            }
        "##,
    );

    let dm2 = parse(
        r##"
            model User {
                id Int @id
                name String @translate("German")
            }
        "##,
    );

    let steps = infer(&dm1, &dm2);

    let locator = ArgumentLocation {
        argument_container: "translate".to_owned(),
        arguments: None,
        argument_type: ArgumentType::FieldDirective {
            model: "User".to_owned(),
            field: "name".to_owned(),
        },
    };

    let expected = &[
        MigrationStep::DeleteArgument(DeleteArgument {
            location: locator.clone(),
            argument: "secondary".to_owned(),
        }),
        MigrationStep::DeleteArgument(DeleteArgument {
            location: locator,
            argument: "tertiary".to_owned(),
        }),
    ];

    assert_eq!(steps, expected);
}

#[test]
fn infer_DeleteArgument_on_model() {
    let dm1 = parse(
        r##"
            model User {
                id Int @id
                name String

                @@randomDirective([name], name: "usernameUniqueness")
            }
        "##,
    );

    let dm2 = parse(
        r##"
            model User {
                id Int @id
                name String

                @@randomDirective([name])
            }
        "##,
    );

    let steps = infer(&dm1, &dm2);

    let locator = ArgumentLocation {
        argument_container: "randomDirective".to_owned(),
        arguments: None,
        argument_type: ArgumentType::ModelDirective {
            model: "User".to_owned(),
        },
    };

    let expected = &[MigrationStep::DeleteArgument(DeleteArgument {
        location: locator,
        argument: "name".to_owned(),
    })];

    assert_eq!(steps, expected);
}

#[test]
fn infer_DeleteArgument_on_enum() {
    let dm1 = parse(
        r##"
            enum EyeColor {
                BLUE
                GREEN
                BROWN

                @@random(one: "two", three: 4)
            }
        "##,
    );

    assert_eq!(infer(&dm1, &dm1), &[]);

    let dm2 = parse(
        r##"
            enum EyeColor {
                BLUE
                GREEN
                BROWN

                @@random(one: "two")
            }
        "##,
    );

    let steps = infer(&dm1, &dm2);

    let locator = ArgumentLocation {
        arguments: None,
        argument_container: "random".to_owned(),
        argument_type: ArgumentType::EnumDirective {
            r#enum: "EyeColor".to_owned(),
        },
    };

    let expected = &[MigrationStep::DeleteArgument(DeleteArgument {
        location: locator,
        argument: "three".to_owned(),
    })];

    assert_eq!(steps, expected);
}

#[test]
fn infer_DeleteArgument_on_type_alias() {
    let dm1 = parse(r#"type BlogPost = String @customDirective(a: "b", c: "d")"#);
    let dm2 = parse(r#"type BlogPost = String @customDirective(c: "d")"#);

    let steps = infer(&dm1, &dm2);

    let locator = ArgumentLocation {
        arguments: None,
        argument_container: "customDirective".to_owned(),
        argument_type: ArgumentType::TypeAlias {
            type_alias: "BlogPost".to_owned(),
        },
    };

    let expected = &[MigrationStep::DeleteArgument(DeleteArgument {
        location: locator,
        argument: "a".to_owned(),
    })];

    assert_eq!(steps, expected);
}

#[test]
fn infer_UpdateArgument_on_field() {
    let dm1 = parse(
        r##"
            model User {
                id Int @id
                name String @translate("German", secondary: "ZH-CN", tertiary: "FR-BE")
            }
        "##,
    );

    let dm2 = parse(
        r##"
            model User {
                id Int @id
                name String @translate("German",  secondary: "FR-BE", tertiary: "ZH-CN")
            }
        "##,
    );

    let steps = infer(&dm1, &dm2);

    let locator = ArgumentLocation {
        argument_container: "translate".to_owned(),
        arguments: None,
        argument_type: ArgumentType::FieldDirective {
            model: "User".to_owned(),
            field: "name".to_owned(),
        },
    };

    let expected = &[
        MigrationStep::UpdateArgument(UpdateArgument {
            location: locator.clone(),
            argument: "secondary".to_owned(),
            new_value: MigrationExpression("\"FR-BE\"".to_owned()),
        }),
        MigrationStep::UpdateArgument(UpdateArgument {
            location: locator,
            argument: "tertiary".to_owned(),
            new_value: MigrationExpression("\"ZH-CN\"".to_owned()),
        }),
    ];

    assert_eq!(steps, expected);
}

#[test]
fn infer_UpdateArgument_on_model() {
    let dm1 = parse(
        r##"
            model User {
                id Int @id
                name String
                nickname String

                @@map("customers")
            }
        "##,
    );

    let dm2 = parse(
        r##"
            model User {
                id Int @id
                name String
                nickname String

                @@map("customers_table")
            }
        "##,
    );

    let steps = infer(&dm1, &dm2);

    let locator = ArgumentLocation {
        argument_container: "map".to_owned(),
        arguments: None,
        argument_type: ArgumentType::ModelDirective {
            model: "User".to_owned(),
        },
    };

    let expected = &[MigrationStep::UpdateArgument(UpdateArgument {
        location: locator,
        argument: "".to_owned(),
        new_value: MigrationExpression("\"customers_table\"".to_owned()),
    })];

    assert_eq!(steps, expected);
}

#[test]
fn infer_UpdateArgument_on_enum() {
    let dm1 = parse(
        r##"
            enum EyeColor {
                BLUE
                GREEN
                BROWN

                @@random(one: "two")
            }
        "##,
    );

    assert_eq!(infer(&dm1, &dm1), &[]);

    let dm2 = parse(
        r##"
            enum EyeColor {
                BLUE
                GREEN
                BROWN

                @@random(one: "three")
            }
        "##,
    );

    let steps = infer(&dm1, &dm2);

    let locator = ArgumentLocation {
        argument_container: "random".to_owned(),
        arguments: None,
        argument_type: ArgumentType::EnumDirective {
            r#enum: "EyeColor".to_owned(),
        },
    };

    let expected = &[MigrationStep::UpdateArgument(UpdateArgument {
        location: locator,
        argument: "one".to_owned(),
        new_value: MigrationExpression("\"three\"".to_owned()),
    })];

    assert_eq!(steps, expected);
}

#[test]
fn infer_UpdateArgument_on_type_alias() {
    let dm1 = parse("type Text = String @default(\"chicken\")");
    let dm2 = parse("type Text = String @default(\"\")");

    let steps = infer(&dm1, &dm2);

    let locator = ArgumentLocation {
        argument_container: "default".to_owned(),
        arguments: None,
        argument_type: ArgumentType::TypeAlias {
            type_alias: "Text".to_owned(),
        },
    };

    let expected = &[MigrationStep::UpdateArgument(UpdateArgument {
        location: locator,
        argument: "".to_owned(),
        new_value: MigrationExpression("\"\"".to_owned()),
    })];

    assert_eq!(steps, expected);
}

#[test]
fn infer_CreateTypeAlias() {
    let dm1 = parse("");
    let dm2 = parse(
        r#"
            type CUID = String @id @default(cuid())

            model User {
                id CUID
                age Float
            }
        "#,
    );

    let steps = infer(&dm1, &dm2);

    let directive_type = ArgumentType::TypeAlias {
        type_alias: "CUID".to_owned(),
    };

    let expected = &[
        MigrationStep::CreateTypeAlias(CreateTypeAlias {
            type_alias: "CUID".to_owned(),
            r#type: "String".to_owned(),
            arity: FieldArity::Required,
        }),
        MigrationStep::CreateArgumentContainer(CreateArgumentContainer {
            location: ArgumentLocation {
                argument_type: directive_type.clone(),
                argument_container: "id".to_owned(),
                arguments: None,
            },
        }),
        MigrationStep::CreateArgumentContainer(CreateArgumentContainer {
            location: ArgumentLocation {
                argument_type: directive_type.clone(),
                argument_container: "default".to_owned(),
                arguments: None,
            },
        }),
        MigrationStep::CreateArgument(CreateArgument {
            location: ArgumentLocation {
                argument_type: directive_type,
                argument_container: "default".to_owned(),
                arguments: None,
            },
            argument: "".to_owned(),
            value: MigrationExpression("cuid()".to_owned()),
        }),
        MigrationStep::CreateModel(CreateModel {
            model: "User".to_string(),
        }),
        MigrationStep::CreateField(CreateField {
            model: "User".to_string(),
            field: "id".to_owned(),
            tpe: "CUID".to_owned(),
            arity: FieldArity::Required,
        }),
        MigrationStep::CreateField(CreateField {
            model: "User".to_string(),
            field: "age".to_owned(),
            tpe: "Float".to_owned(),
            arity: FieldArity::Required,
        }),
    ];

    assert_eq!(steps, expected);
}

#[test]
fn infer_DeleteTypeAlias() {
    let dm1 = parse("type CUID = String @id @default(cuid())");
    let dm2 = parse("");
    let steps = infer(&dm1, &dm2);

    let expected = &[MigrationStep::DeleteTypeAlias(DeleteTypeAlias {
        type_alias: "CUID".to_owned(),
    })];

    assert_eq!(steps, expected);
}

#[test]
fn infer_UpdateTypeAlias() {
    let dm1 = parse("type Age = Int");
    let dm2 = parse("type Age = Float");

    let steps = infer(&dm1, &dm2);

    let expected = &[MigrationStep::UpdateTypeAlias(UpdateTypeAlias {
        type_alias: "Age".to_owned(),
        r#type: Some("Float".to_owned()),
    })];

    assert_eq!(steps, expected);
}

fn infer(dm1: &SchemaAst, dm2: &SchemaAst) -> Vec<MigrationStep> {
    let inferrer = DataModelMigrationStepsInferrerImplWrapper {};
    inferrer.infer(&dm1, &dm2)
}

fn parse(input: &str) -> SchemaAst {
    parser::parse(input).unwrap()
}
