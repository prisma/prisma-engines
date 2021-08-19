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
fn on_delete_cannot_be_defined_on_the_wrong_side_1_n() {
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
fn on_update_cannot_be_defined_on_the_wrong_side_1_n() {
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
fn on_delete_cannot_be_defined_on_the_wrong_side_1_1() {
    let dml = indoc! { r#"
        datasource db {
            provider = "mysql"
            url = "mysql://"
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["referentialActions"]
        }

        model Chicken {
            id        Int      @id @default(autoincrement())
            cock      Chicken? @relation(name: "a_self_relation", onDelete: NoAction)
            hen       Chicken? @relation(name: "a_self_relation", fields: [chickenId], references: [id])
            chickenId Int?
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation field `cock` on Model `Chicken` must not specify the `onDelete` or `onUpdate` argument in the @relation attribute. You must only specify it on the opposite field `hen` on model `Chicken`.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m    id        Int      @id @default(autoincrement())
        [1;94m13 | [0m    [1;91mcock      Chicken? @relation(name: "a_self_relation", onDelete: NoAction)[0m
        [1;94m14 | [0m    hen       Chicken? @relation(name: "a_self_relation", fields: [chickenId], references: [id])
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&datamodel::parse_schema(dml).map(drop).unwrap_err());
}

#[test]
fn on_update_cannot_be_defined_on_the_wrong_side_1_1() {
    let dml = indoc! { r#"
        datasource db {
            provider = "mysql"
            url = "mysql://"
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["referentialActions"]
        }

        model Chicken {
            id        Int      @id @default(autoincrement())
            cock      Chicken? @relation(name: "a_self_relation", onUpdate: NoAction)
            hen       Chicken? @relation(name: "a_self_relation", fields: [chickenId], references: [id])
            chickenId Int?
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation field `cock` on Model `Chicken` must not specify the `onDelete` or `onUpdate` argument in the @relation attribute. You must only specify it on the opposite field `hen` on model `Chicken`.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m    id        Int      @id @default(autoincrement())
        [1;94m13 | [0m    [1;91mcock      Chicken? @relation(name: "a_self_relation", onUpdate: NoAction)[0m
        [1;94m14 | [0m    hen       Chicken? @relation(name: "a_self_relation", fields: [chickenId], references: [id])
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&datamodel::parse_schema(dml).map(drop).unwrap_err());
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

#[test]
fn sql_server_cascading_on_delete_self_relations() {
    let dml = indoc! {
        r#"
        datasource db {
            provider = "sqlserver"
            url = "sqlserver://"
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["referentialActions", "microsoftSqlServer"]
        }

        model A {
            id     Int  @id @default(autoincrement())
            child  A?   @relation(name: "a_self_relation")
            parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onDelete: Cascade)
            aId    Int?
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes. Implicit default `onDelete` and `onUpdate` values: `SetNull` and `Cascade`. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m    id     Int  @id @default(autoincrement())
        [1;94m13 | [0m    [1;91mchild  A?   @relation(name: "a_self_relation")[0m
        [1;94m14 | [0m    parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onDelete: Cascade)
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes. Implicit default `onUpdate` value: `Cascade`. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m    child  A?   @relation(name: "a_self_relation")
        [1;94m14 | [0m    [1;91mparent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onDelete: Cascade)[0m
        [1;94m15 | [0m    aId    Int?
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&datamodel::parse_schema(dml).map(drop).unwrap_err());
}

#[test]
fn sql_server_cascading_on_update_self_relations() {
    let dml = indoc! {
        r#"
        datasource db {
            provider = "sqlserver"
            url = "sqlserver://"
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["referentialActions", "microsoftSqlServer"]
        }

        model A {
            id     Int  @id @default(autoincrement())
            child  A?   @relation(name: "a_self_relation")
            parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onUpdate: Cascade)
            aId    Int?
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes. Implicit default `onDelete` and `onUpdate` values: `SetNull` and `Cascade`. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m    id     Int  @id @default(autoincrement())
        [1;94m13 | [0m    [1;91mchild  A?   @relation(name: "a_self_relation")[0m
        [1;94m14 | [0m    parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onUpdate: Cascade)
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes. Implicit default `onDelete` value: `SetNull`. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m    child  A?   @relation(name: "a_self_relation")
        [1;94m14 | [0m    [1;91mparent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onUpdate: Cascade)[0m
        [1;94m15 | [0m    aId    Int?
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&datamodel::parse_schema(dml).map(drop).unwrap_err());
}

#[test]
fn sql_server_null_setting_on_delete_self_relations() {
    let dml = indoc! {
        r#"
        datasource db {
            provider = "sqlserver"
            url = "sqlserver://"
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["referentialActions", "microsoftSqlServer"]
        }

        model A {
            id     Int  @id @default(autoincrement())
            child  A?   @relation(name: "a_self_relation")
            parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onDelete: SetNull)
            aId    Int?
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes. Implicit default `onDelete` and `onUpdate` values: `SetNull` and `Cascade`. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m    id     Int  @id @default(autoincrement())
        [1;94m13 | [0m    [1;91mchild  A?   @relation(name: "a_self_relation")[0m
        [1;94m14 | [0m    parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onDelete: SetNull)
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes. Implicit default `onUpdate` value: `Cascade`. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m    child  A?   @relation(name: "a_self_relation")
        [1;94m14 | [0m    [1;91mparent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onDelete: SetNull)[0m
        [1;94m15 | [0m    aId    Int?
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&datamodel::parse_schema(dml).map(drop).unwrap_err());
}

#[test]
fn sql_server_null_setting_on_update_self_relations() {
    let dml = indoc! {
        r#"
        datasource db {
            provider = "sqlserver"
            url = "sqlserver://"
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["referentialActions", "microsoftSqlServer"]
        }

        model A {
            id     Int  @id @default(autoincrement())
            child  A?   @relation(name: "a_self_relation")
            parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onUpdate: SetNull)
            aId    Int?
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes. Implicit default `onDelete` and `onUpdate` values: `SetNull` and `Cascade`. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m    id     Int  @id @default(autoincrement())
        [1;94m13 | [0m    [1;91mchild  A?   @relation(name: "a_self_relation")[0m
        [1;94m14 | [0m    parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onUpdate: SetNull)
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes. Implicit default `onDelete` value: `SetNull`. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m    child  A?   @relation(name: "a_self_relation")
        [1;94m14 | [0m    [1;91mparent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onUpdate: SetNull)[0m
        [1;94m15 | [0m    aId    Int?
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&datamodel::parse_schema(dml).map(drop).unwrap_err());
}

#[test]
fn sql_server_default_setting_on_delete_self_relations() {
    let dml = indoc! {
        r#"
        datasource db {
            provider = "sqlserver"
            url = "sqlserver://"
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["referentialActions", "microsoftSqlServer"]
        }

        model A {
            id     Int  @id @default(autoincrement())
            child  A?   @relation(name: "a_self_relation")
            parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onDelete: SetDefault)
            aId    Int?
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes. Implicit default `onDelete` and `onUpdate` values: `SetNull` and `Cascade`. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m    id     Int  @id @default(autoincrement())
        [1;94m13 | [0m    [1;91mchild  A?   @relation(name: "a_self_relation")[0m
        [1;94m14 | [0m    parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onDelete: SetDefault)
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes. Implicit default `onUpdate` value: `Cascade`. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m    child  A?   @relation(name: "a_self_relation")
        [1;94m14 | [0m    [1;91mparent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onDelete: SetDefault)[0m
        [1;94m15 | [0m    aId    Int?
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&datamodel::parse_schema(dml).map(drop).unwrap_err());
}

#[test]
fn sql_server_default_setting_on_update_self_relations() {
    let dml = indoc! {
        r#"
        datasource db {
            provider = "sqlserver"
            url = "sqlserver://"
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["referentialActions", "microsoftSqlServer"]
        }

        model A {
            id     Int  @id @default(autoincrement())
            child  A?   @relation(name: "a_self_relation")
            parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onUpdate: SetDefault)
            aId    Int?
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes. Implicit default `onDelete` and `onUpdate` values: `SetNull` and `Cascade`. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m    id     Int  @id @default(autoincrement())
        [1;94m13 | [0m    [1;91mchild  A?   @relation(name: "a_self_relation")[0m
        [1;94m14 | [0m    parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onUpdate: SetDefault)
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes. Implicit default `onDelete` value: `SetNull`. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m    child  A?   @relation(name: "a_self_relation")
        [1;94m14 | [0m    [1;91mparent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onUpdate: SetDefault)[0m
        [1;94m15 | [0m    aId    Int?
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&datamodel::parse_schema(dml).map(drop).unwrap_err());
}

#[test]
fn sql_server_cascading_cyclic_one_hop_relations() {
    let dml = indoc! {
        r#"
        datasource db {
            provider = "sqlserver"
            url = "sqlserver://"
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["referentialActions", "microsoftSqlServer"]
        }

        model A {
            id     Int  @id @default(autoincrement())
            b      B    @relation(name: "foo", fields: [bId], references: [id], onDelete: Cascade)
            bId    Int
            bs     B[]  @relation(name: "bar")
        }

        model B {
            id     Int @id @default(autoincrement())
            a      A   @relation(name: "bar", fields: [aId], references: [id], onUpdate: Cascade)
            as     A[] @relation(name: "foo")
            aId    Int
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": Reference causes a cycle or multiple cascade paths. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: A.b → B.a. Implicit default `onUpdate` value: `Cascade`. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m    id     Int  @id @default(autoincrement())
        [1;94m13 | [0m    [1;91mb      B    @relation(name: "foo", fields: [bId], references: [id], onDelete: Cascade)[0m
        [1;94m14 | [0m    bId    Int
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": Reference causes a cycle or multiple cascade paths. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: B.a → A.b. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:20[0m
        [1;94m   | [0m
        [1;94m19 | [0m    id     Int @id @default(autoincrement())
        [1;94m20 | [0m    [1;91ma      A   @relation(name: "bar", fields: [aId], references: [id], onUpdate: Cascade)[0m
        [1;94m21 | [0m    as     A[] @relation(name: "foo")
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&datamodel::parse_schema(dml).map(drop).unwrap_err());
}

#[test]
fn sql_server_cascading_cyclic_hop_over_table_relations() {
    let dml = indoc! {
        r#"
        datasource db {
            provider = "sqlserver"
            url = "sqlserver://"
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["referentialActions", "microsoftSqlServer"]
        }

        model A {
            id     Int  @id @default(autoincrement())
            bId    Int
            b      B    @relation(fields: [bId], references: [id])
            cs     C[]
        }

        model B {
            id     Int  @id @default(autoincrement())
            as     A[]
            cId    Int
            c      C    @relation(fields: [cId], references: [id])
        }

        model C {
            id     Int @id @default(autoincrement())
            bs     B[]
            aId    Int
            a      A   @relation(fields: [aId], references: [id])
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": Reference causes a cycle or multiple cascade paths. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: A.b → B.c → C.a. Implicit default `onUpdate` value: `Cascade`. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m    bId    Int
        [1;94m14 | [0m    [1;91mb      B    @relation(fields: [bId], references: [id])[0m
        [1;94m15 | [0m    cs     C[]
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": Reference causes a cycle or multiple cascade paths. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: B.c → C.a → A.b. Implicit default `onUpdate` value: `Cascade`. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:22[0m
        [1;94m   | [0m
        [1;94m21 | [0m    cId    Int
        [1;94m22 | [0m    [1;91mc      C    @relation(fields: [cId], references: [id])[0m
        [1;94m23 | [0m}
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": Reference causes a cycle or multiple cascade paths. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: C.a → A.b → B.c. Implicit default `onUpdate` value: `Cascade`. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:29[0m
        [1;94m   | [0m
        [1;94m28 | [0m    aId    Int
        [1;94m29 | [0m    [1;91ma      A   @relation(fields: [aId], references: [id])[0m
        [1;94m30 | [0m}
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&datamodel::parse_schema(dml).map(drop).unwrap_err());
}

#[test]
fn sql_server_cascading_cyclic_hop_over_backrelation() {
    let dml = indoc! {
        r#"
        datasource test {
            provider = "sqlserver"
            url      = "sqlserver://localhost:1433;database=master;user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true"
        }

        generator client {
            provider        = "prisma-client-js"
            previewFeatures = ["microsoftSqlServer", "referentialActions"]
        }

        model User {
            id        Int       @id @default(autoincrement())
            comments  Comment[]
            posts     Post[]
        }

        model Post {
            id        Int       @id @default(autoincrement())
            authorId  Int
            author    User      @relation(fields: [authorId], references: [id])
            comments  Comment[]
            tags      Tag[]     @relation("TagToPost")
        }

        model Comment {
            id          Int      @id @default(autoincrement())
            writtenById Int
            postId      Int
            writtenBy   User     @relation(fields: [writtenById], references: [id])
            post        Post     @relation(fields: [postId], references: [id])
        }

        model Tag {
            id    Int    @id @default(autoincrement())
            tag   String @unique
            posts Post[] @relation("TagToPost")
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": Reference causes a cycle or multiple cascade paths. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: Post.author → User.comments → Comment.post. Implicit default `onUpdate` value: `Cascade`. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:20[0m
        [1;94m   | [0m
        [1;94m19 | [0m    authorId  Int
        [1;94m20 | [0m    [1;91mauthor    User      @relation(fields: [authorId], references: [id])[0m
        [1;94m21 | [0m    comments  Comment[]
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": Reference causes a cycle or multiple cascade paths. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: Comment.writtenBy → User.posts → Post.comments. Implicit default `onUpdate` value: `Cascade`. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:29[0m
        [1;94m   | [0m
        [1;94m28 | [0m    postId      Int
        [1;94m29 | [0m    [1;91mwrittenBy   User     @relation(fields: [writtenById], references: [id])[0m
        [1;94m30 | [0m    post        Post     @relation(fields: [postId], references: [id])
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": Reference causes a cycle or multiple cascade paths. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: Comment.post → Post.author → User.comments. Implicit default `onUpdate` value: `Cascade`. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:30[0m
        [1;94m   | [0m
        [1;94m29 | [0m    writtenBy   User     @relation(fields: [writtenById], references: [id])
        [1;94m30 | [0m    [1;91mpost        Post     @relation(fields: [postId], references: [id])[0m
        [1;94m31 | [0m}
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&datamodel::parse_schema(dml).map(drop).unwrap_err());
}

#[test]
fn sql_server_cascading_cyclic_crossing_path_relations() {
    let dml = indoc! {
        r#"
        datasource db {
            provider = "sqlserver"
            url = "sqlserver://"
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["referentialActions", "microsoftSqlServer"]
        }

        model A {
            id     Int  @id @default(autoincrement())
            bId    Int
            b      B    @relation(fields: [bId], references: [id])
            cs     C[]
        }

        model B {
            id     Int  @id @default(autoincrement())
            as     A[]
            cs     C[]
        }

        model C {
            id     Int  @id @default(autoincrement())
            aId    Int
            bId    Int
            a      A    @relation(fields: [aId], references: [id])
            b      B    @relation(fields: [bId], references: [id])
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": Reference causes a cycle or multiple cascade paths. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: A.b → B.cs → C.a. Implicit default `onUpdate` value: `Cascade`. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m    bId    Int
        [1;94m14 | [0m    [1;91mb      B    @relation(fields: [bId], references: [id])[0m
        [1;94m15 | [0m    cs     C[]
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": Reference causes a cycle or multiple cascade paths. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: C.a → A.b → B.cs. Implicit default `onUpdate` value: `Cascade`. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:28[0m
        [1;94m   | [0m
        [1;94m27 | [0m    bId    Int
        [1;94m28 | [0m    [1;91ma      A    @relation(fields: [aId], references: [id])[0m
        [1;94m29 | [0m    b      B    @relation(fields: [bId], references: [id])
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": Reference causes a cycle or multiple cascade paths. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: C.b → B.as → A.cs. Implicit default `onUpdate` value: `Cascade`. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:29[0m
        [1;94m   | [0m
        [1;94m28 | [0m    a      A    @relation(fields: [aId], references: [id])
        [1;94m29 | [0m    [1;91mb      B    @relation(fields: [bId], references: [id])[0m
        [1;94m30 | [0m}
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&datamodel::parse_schema(dml).map(drop).unwrap_err());
}

#[test]
fn cycle_detection_prints_the_right_path() {
    let dm = r#"
    datasource db {
        provider = "sqlserver"
        url = "sqlserver://"
    }

    generator client {
        provider = "prisma-client-js"
        previewFeatures = ["referentialActions", "microsoftSqlServer"]
    }

    model Post {
        id       Int       @id @default(autoincrement())
        user_id  Int       @map("bId")
        user     User      @relation(fields: [user_id], references: [id])
        comments Comment[]
        @@map("A")
    }

    model User {
        id         Int     @id @default(autoincrement())
        posts      Post[]
        address_id Int
        comment_id Int     @map("cId")
        address    Address @relation(fields: [address_id], references: [id])
        comment    Comment @relation(fields: [comment_id], references: [id])
        @@map("B")
    }

    model Address {
        id Int @id @default(autoincrement())
        sId Int
        something Something @relation(fields: [sId], references: [id])
        users User[]
    }

    model Something {
        id Int @id @default(autoincrement())
        oId Int
        other Other @relation(fields: [oId], references: [id])
        addresses Address[]
    }

    model Other {
        id Int @id @default(autoincrement())
        somethings Something[]
    }

    model Comment {
        id      Int    @id @default(autoincrement())
        users   User[]
        post_id Int    @map("aId")
        post    Post   @relation(fields: [post_id], references: [id])
        @@map("C")
    }
    "#;

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": Reference causes a cycle or multiple cascade paths. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: Post.user → User.comment → Comment.post. Implicit default `onUpdate` value: `Cascade`. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m        user_id  Int       @map("bId")
        [1;94m15 | [0m        [1;91muser     User      @relation(fields: [user_id], references: [id])[0m
        [1;94m16 | [0m        comments Comment[]
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": Reference causes a cycle or multiple cascade paths. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: User.comment → Comment.post → Post.user. Implicit default `onUpdate` value: `Cascade`. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:26[0m
        [1;94m   | [0m
        [1;94m25 | [0m        address    Address @relation(fields: [address_id], references: [id])
        [1;94m26 | [0m        [1;91mcomment    Comment @relation(fields: [comment_id], references: [id])[0m
        [1;94m27 | [0m        @@map("B")
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": Reference causes a cycle or multiple cascade paths. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: Comment.post → Post.user → User.comment. Implicit default `onUpdate` value: `Cascade`. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:53[0m
        [1;94m   | [0m
        [1;94m52 | [0m        post_id Int    @map("aId")
        [1;94m53 | [0m        [1;91mpost    Post   @relation(fields: [post_id], references: [id])[0m
        [1;94m54 | [0m        @@map("C")
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&datamodel::parse_schema(dm).map(drop).unwrap_err());
}
