use crate::common::*;
use datamodel::{ast::Span, diagnostics::DatamodelError, ReferentialAction::*};
use indoc::{formatdoc, indoc};

#[test]
fn on_delete_actions() {
    let actions = &[Cascade, Restrict, NoAction, SetNull, SetDefault];

    for action in actions {
        let dml = formatdoc!(
            r#"
            model A {{
                id Int @id
                bs B[]
            }}

            model B {{
                id Int @id
                aId Int
                a A @relation(fields: [aId], references: [id], onDelete: {})
            }}
        "#,
            action
        );

        parse(&dml)
            .assert_has_model("B")
            .assert_has_relation_field("a")
            .assert_relation_delete_strategy(*action);
    }
}

#[test]
fn on_update_actions() {
    let actions = &[Cascade, Restrict, NoAction, SetNull, SetDefault];

    for action in actions {
        let dml = formatdoc!(
            r#"
            model A {{
                id Int @id
                bs B[]
            }}

            model B {{
                id Int @id
                aId Int
                a A @relation(fields: [aId], references: [id], onUpdate: {})
            }}
        "#,
            action
        );

        parse(&dml)
            .assert_has_model("B")
            .assert_has_relation_field("a")
            .assert_relation_update_strategy(*action);
    }
}

#[test]
fn actions_on_mongo() {
    let actions = &[Restrict, SetNull];

    for action in actions {
        let dml = formatdoc!(
            r#"
            datasource db {{
                provider = "mongodb"
                url = "mongodb://"
            }}

            model A {{
                id Int @id @map("_id")
                bs B[]
            }}

            model B {{
                id Int @id @map("_id")
                aId Int
                a A @relation(fields: [aId], references: [id], onDelete: {action}, onUpdate: {action})
            }}
        "#,
            action = action
        );

        parse(&dml)
            .assert_has_model("B")
            .assert_has_relation_field("a")
            .assert_relation_delete_strategy(*action)
            .assert_relation_update_strategy(*action);
    }
}

#[test]
fn actions_on_planetscale() {
    let actions = &[Restrict, SetNull];

    for action in actions {
        let dml = formatdoc!(
            r#"
            datasource db {{
                provider = "mysql"
                planetScaleMode = true
                url = "mysql://root:prisma@localhost:3306/mydb"
            }}

            generator client {{
                provider = "prisma-client-js"
                previewFeatures = ["planetScaleMode"]
            }}

            model A {{
                id Int @id
                bs B[]
            }}

            model B {{
                id Int @id
                aId Int
                a A @relation(fields: [aId], references: [id], onDelete: {action}, onUpdate: {action})
            }}
        "#,
            action = action
        );

        parse(&dml)
            .assert_has_model("B")
            .assert_has_relation_field("a")
            .assert_relation_delete_strategy(*action)
            .assert_relation_update_strategy(*action);
    }
}

#[test]
fn invalid_on_delete_action() {
    let dml = indoc! { r#"
        model A {
            id Int @id
            bs B[]
        }

        model B {
            id Int @id
            aId Int
            a A @relation(fields: [aId], references: [id], onDelete: MeowMeow)
        }
    "#};

    parse_error(dml).assert_is(DatamodelError::new_attribute_validation_error(
        "Invalid referential action: `MeowMeow`",
        "relation",
        Span::new(137, 145),
    ));
}

#[test]
fn invalid_on_update_action() {
    let dml = indoc! { r#"
        model A {
            id Int @id
            bs B[]
        }

        model B {
            id Int @id
            aId Int
            a A @relation(fields: [aId], references: [id], onUpdate: MeowMeow)
        }
    "#};

    parse_error(dml).assert_is(DatamodelError::new_attribute_validation_error(
        "Invalid referential action: `MeowMeow`",
        "relation",
        Span::new(137, 145),
    ));
}

#[test]
fn restrict_should_not_work_on_sql_server() {
    let dml = indoc! { r#"
        datasource db {
            provider = "sqlserver"
            url = "sqlserver://"
        }

        model A {
            id Int @id
            bs B[]
        }

        model B {
            id Int @id
            aId Int
            a A @relation(fields: [aId], references: [id], onUpdate: Restrict, onDelete: Restrict)
        }
    "#};

    let message =
        "Invalid referential action: `Restrict`. Allowed values: (`Cascade`, `NoAction`, `SetNull`, `SetDefault`)";

    parse_error(dml).assert_are(&[
        DatamodelError::new_attribute_validation_error(&message, "relation", Span::new(151, 238)),
        DatamodelError::new_attribute_validation_error(&message, "relation", Span::new(151, 238)),
    ]);
}

#[test]
fn concrete_actions_should_not_work_on_mongo() {
    let actions = &[(Cascade, 237), (NoAction, 238), (SetDefault, 240)];

    for (action, span) in actions {
        let dml = formatdoc!(
            r#"
            datasource db {{
                provider = "mongodb"
                url = "mongodb://"
            }}

            model A {{
                id Int @id @map("_id")
                bs B[]
            }}

            model B {{
                id Int @id @map("_id")
                aId Int
                a A @relation(fields: [aId], references: [id], onDelete: {})
            }}
        "#,
            action
        );

        let message = format!(
            "Invalid referential action: `{}`. Allowed values: (`Restrict`, `SetNull`)",
            action
        );

        parse_error(&dml).assert_are(&[DatamodelError::new_attribute_validation_error(
            &message,
            "relation",
            Span::new(171, *span),
        )]);
    }
}

#[test]
fn concrete_actions_should_not_work_on_planetscale() {
    let actions = &[(Cascade, 389), (NoAction, 390), (SetDefault, 392)];

    for (action, span) in actions {
        let dml = formatdoc!(
            r#"
            datasource db {{
                provider = "mysql"
                planetScaleMode = true
                url = "mysql://root:prisma@localhost:3306/mydb"
            }}

            generator client {{
                provider = "prisma-client-js"
                previewFeatures = ["planetScaleMode"]
            }}

            model A {{
                id Int @id @map("_id")
                bs B[]
            }}

            model B {{
                id Int @id @map("_id")
                aId Int
                a A @relation(fields: [aId], references: [id], onDelete: {})
            }}
        "#,
            action
        );

        let message = format!(
            "Invalid referential action: `{}`. Allowed values: (`Restrict`, `SetNull`)",
            action
        );

        parse_error(&dml).assert_are(&[DatamodelError::new_attribute_validation_error(
            &message,
            "relation",
            Span::new(323, *span),
        )]);
    }
}

#[test]
fn on_delete_cannot_be_defined_on_the_wrong_side() {
    let dml = indoc! { r#"
        datasource db {
            provider = "mysql"
            url = "mysql://"
        }

        model A {
            id Int @id
            bs B[] @relation(onDelete: Restrict)
        }

        model B {
            id Int @id
            aId Int
            a A @relation(fields: [aId], references: [id], onDelete: Restrict)
        }
    "#};

    let message =
        "The relation field `bs` on Model `A` must not specify the `onDelete` or `onUpdate` argument in the @relation attribute. You must only specify it on the opposite field `a` on model `B`, or in case of a many to many relation, in an explicit join table.";

    parse_error(dml).assert_are(&[DatamodelError::new_attribute_validation_error(
        &message,
        "relation",
        Span::new(92, 129),
    )]);
}
