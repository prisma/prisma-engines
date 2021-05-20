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
fn nothing_should_work_on_mongo() {
    let dml = indoc! {r#"
        datasource db {
            provider = "mongodb"
            url = "mongodb://"
        }

        model A {
            id Int @id @map("_id")
            bs B[]
        }

        model B {
            id Int @id @map("_id")
            aId Int
            a A @relation(fields: [aId], references: [id], onUpdate: Cascade, onDelete: Cascade)
        }
    "#};

    let message = "Referential actions are not supported for current connector.";

    parse_error(&dml).assert_are(&[
        DatamodelError::new_attribute_validation_error(&message, "relation", Span::new(171, 256)),
        DatamodelError::new_attribute_validation_error(&message, "relation", Span::new(171, 256)),
    ]);
}
