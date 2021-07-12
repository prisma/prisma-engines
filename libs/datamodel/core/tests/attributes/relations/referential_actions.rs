use crate::common::*;
use datamodel::{ast::Span, diagnostics::DatamodelError, ReferentialAction::*};
use indoc::{formatdoc, indoc};

#[test]
fn on_delete_actions() {
    let actions = &[Cascade, Restrict, NoAction, SetNull, SetDefault];

    for action in actions {
        let dml = formatdoc!(
            r#"
            generator client {{
                provider = "prisma-client-js"
                previewFeatures = ["referentialActions"]
            }}

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
            generator client {{
                provider = "prisma-client-js"
                previewFeatures = ["referentialActions"]
            }}

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

            generator client {{
                provider = "prisma-client-js"
                previewFeatures = ["referentialActions"]
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
                previewFeatures = ["planetScaleMode", "referentialActions"]
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
        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["referentialActions"]
        }

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
        Span::new(238, 246),
    ));
}

#[test]
fn invalid_on_update_action() {
    let dml = indoc! { r#"
        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["referentialActions"]
        }

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
        Span::new(238, 246),
    ));
}

#[test]
fn restrict_should_not_work_on_sql_server() {
    let dml = indoc! { r#"
        datasource db {
            provider = "sqlserver"
            url = "sqlserver://"
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["referentialActions"]
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
        DatamodelError::new_attribute_validation_error(message, "relation", Span::new(252, 339)),
        DatamodelError::new_attribute_validation_error(message, "relation", Span::new(252, 339)),
    ]);
}

#[test]
fn actions_should_be_defined_only_from_one_side() {
    let dml = indoc! { r#"
        datasource db {
            provider = "sqlserver"
            url = "sqlserver://"
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["referentialActions"]
        }

        model A {
            id Int @id
            b B? @relation(onUpdate: NoAction, onDelete: NoAction)
        }

        model B {
            id Int @id
            aId Int
            a A @relation(fields: [aId], references: [id], onUpdate: NoAction, onDelete: NoAction)
        }
    "#};

    let message1 =
        "The relation fields `b` on Model `A` and `a` on Model `B` both provide the `onDelete` or `onUpdate` argument in the @relation attribute. You have to provide it only on one of the two fields.";

    let message2 =
        "The relation fields `a` on Model `B` and `b` on Model `A` both provide the `onDelete` or `onUpdate` argument in the @relation attribute. You have to provide it only on one of the two fields.";

    parse_error(dml).assert_are(&[
        DatamodelError::new_attribute_validation_error(message1, "relation", Span::new(201, 256)),
        DatamodelError::new_attribute_validation_error(message2, "relation", Span::new(300, 387)),
    ]);
}

#[test]
fn concrete_actions_should_not_work_on_planetscale() {
    let actions = &[(Cascade, 411), (NoAction, 412), (SetDefault, 414)];

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
                previewFeatures = ["planetScaleMode", "referentialActions"]
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
            Span::new(345, *span),
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

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["referentialActions"]
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
        message,
        "relation",
        Span::new(193, 230),
    )]);
}

#[test]
fn on_update_cannot_be_defined_on_the_wrong_side() {
    let dml = indoc! { r#"
        datasource db {
            provider = "mysql"
            url = "mysql://"
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["referentialActions"]
        }

        model A {
            id Int @id
            bs B[] @relation(onUpdate: Restrict)
        }

        model B {
            id Int @id
            aId Int
            a A @relation(fields: [aId], references: [id], onUpdate: Restrict)
        }
    "#};

    let message =
        "The relation field `bs` on Model `A` must not specify the `onDelete` or `onUpdate` argument in the @relation attribute. You must only specify it on the opposite field `a` on model `B`, or in case of a many to many relation, in an explicit join table.";

    parse_error(dml).assert_are(&[DatamodelError::new_attribute_validation_error(
        message,
        "relation",
        Span::new(193, 230),
    )]);
}

#[test]
fn on_delete_without_preview_feature_should_error() {
    let dml = indoc! { r#"
        model A {
            id Int @id
            bs B[]
        }

        model B {
            id Int @id
            aId Int
            a A @relation(fields: [aId], references: [id], onDelete: Restrict)
        }
    "#};

    let message = "The relation field `a` on Model `B` must not specify the `onDelete` argument in the @relation attribute without enabling the `referentialActions` preview feature.";

    parse_error(dml).assert_are(&[DatamodelError::new_attribute_validation_error(
        message,
        "relation",
        Span::new(127, 145),
    )]);
}

#[test]
fn on_update_without_preview_feature_should_error() {
    let dml = indoc! { r#"
        model A {
            id Int @id
            bs B[]
        }

        model B {
            id Int @id
            aId Int
            a A @relation(fields: [aId], references: [id], onUpdate: Restrict)
        }
    "#};

    let message = "The relation field `a` on Model `B` must not specify the `onUpdate` argument in the @relation attribute without enabling the `referentialActions` preview feature.";

    parse_error(dml).assert_are(&[DatamodelError::new_attribute_validation_error(
        message,
        "relation",
        Span::new(127, 145),
    )]);
}
