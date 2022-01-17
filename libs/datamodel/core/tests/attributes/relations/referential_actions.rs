mod cycle_detection;

use crate::{common::*, config::parse_config};
use datamodel::ReferentialAction::{self, *};
use datamodel_connector::ReferentialIntegrity;
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
    let actions = &[Restrict, SetNull, Cascade, NoAction];

    for action in actions {
        let dml = formatdoc!(
            r#"
            datasource db {{
                provider = "mongodb"
                url = "mongodb://"
            }}

            generator client {{
                provider = "prisma-client-js"
                previewFeatures = "mongodb"
            }}

            model A {{
                id Int @id @map("_id")
                bs B[]
            }}

            model B {{
                id Int @id @map("_id")
                aId Int
                a A @relation(fields: [aId], references: [id], onDelete: {action})
            }}
        "#,
            action = action
        );

        parse(&dml)
            .assert_has_model("B")
            .assert_has_relation_field("a")
            .assert_relation_delete_strategy(*action);
    }
}

#[test]
fn on_delete_actions_should_work_on_prisma_referential_integrity() {
    let actions = &[Restrict, SetNull, Cascade, NoAction];

    for action in actions {
        let dml = formatdoc!(
            r#"
            datasource db {{
                provider = "mysql"
                referentialIntegrity = "prisma"
                url = "mysql://root:prisma@localhost:3306/mydb"
            }}

            generator client {{
                provider = "prisma-client-js"
                previewFeatures = ["referentialIntegrity"]
            }}

            model A {{
                id Int @id
                bs B[]
            }}

            model B {{
                id Int @id
                aId Int
                a A @relation(fields: [aId], references: [id], onDelete: {action})
            }}
        "#,
            action = action
        );

        parse(&dml)
            .assert_has_model("B")
            .assert_has_relation_field("a")
            .assert_relation_delete_strategy(*action);
    }
}

#[test]
fn on_update_no_action_should_work_on_prisma_referential_integrity() {
    let dml = indoc! { r#"
        datasource db {
          provider = "mysql"
          url = "mysql://"
          referentialIntegrity = "prisma"
        }

        generator client {
          provider = "prisma-client-js"
          previewFeatures = ["referentialIntegrity"]
        }

        model A {
          id Int @id
          bs B[]
        }

        model B {
          id Int @id
          aId Int
          a A @relation(fields: [aId], references: [id], onUpdate: NoAction)
        }
    "#};

    parse(dml)
        .assert_has_model("B")
        .assert_has_relation_field("a")
        .assert_relation_update_strategy(ReferentialAction::NoAction);
}

#[test]
fn foreign_keys_not_allowed_on_mongo() {
    let dml = indoc! {r#"
        datasource db {
          provider = "mongodb"
          referentialIntegrity = "foreignKeys"
          url = "mongodb://"
        }

        generator client {
          provider = "prisma-client-js"
          previewFeatures = ["referentialIntegrity", "mongodb"]
        }

        model A {
          id Int @id
          bs B[]
        }

        model B {
          id Int @id
          aId Int
          a A @relation(fields: [aId], references: [id])
        }
    "#};

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating datasource `referentialIntegrity`: Invalid referential integrity setting: "foreignKeys". Supported values: "prisma"[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "mongodb"
        [1;94m 3 | [0m  referentialIntegrity = [1;91m"foreignKeys"[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&parse_config(dml).map(drop).unwrap_err())
}

#[test]
fn prisma_level_integrity_should_be_allowed_on_mongo() {
    let dml = indoc! {r#"
        datasource db {
          provider = "mongodb"
          referentialIntegrity = "prisma"
          url = "mongodb://"
        }

        generator client {
          provider = "prisma-client-js"
          previewFeatures = ["referentialIntegrity", "mongodb"]
        }

        model A {
          id Int @id
          bs B[]
        }

        model B {
          id Int @id
          aId Int
          a A @relation(fields: [aId], references: [id])
        }
    "#};

    assert!(parse_config(dml).is_ok());
}

#[test]
fn mongo_uses_prisma_referential_integrity_by_default() {
    let dml = indoc! {r#"
        datasource db {
          provider = "mongodb"
          url = "mongodb://"
        }

        generator client {
          provider = "prisma-client-js"
          previewFeatures = ["mongodb"]
        }

        model A {
          id Int @id
          bs B[]
        }

        model B {
          id Int @id
          aId Int
          a A @relation(fields: [aId], references: [id])
        }
    "#};

    assert_eq!(
        Some(ReferentialIntegrity::Prisma),
        parse_config(dml).unwrap().subject.referential_integrity()
    );
}

#[test]
fn sql_databases_use_foreign_keys_referential_integrity_by_default() {
    for db in ["postgres", "mysql", "sqlserver", "sqlite"] {
        let dml = formatdoc! {r#"
            datasource db {{
              provider = "{db}"
              url = "{db}://"
            }}

            model A {{
              id Int @id
              bs B[]
            }}

            model B {{
              id Int @id
              aId Int
              a A @relation(fields: [aId], references: [id])
            }}
        "#, db = db};

        assert_eq!(
            Some(ReferentialIntegrity::ForeignKeys),
            parse_config(&dml).unwrap().subject.referential_integrity()
        );
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

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": Invalid referential action: `MeowMeow`[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m    aId Int
        [1;94m 9 | [0m    a A @relation(fields: [aId], references: [id], onDelete: [1;91mMeowMeow[0m)
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&datamodel::parse_schema(dml).map(drop).unwrap_err());
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

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": Invalid referential action: `MeowMeow`[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m    aId Int
        [1;94m 9 | [0m    a A @relation(fields: [aId], references: [id], onUpdate: [1;91mMeowMeow[0m)
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&datamodel::parse_schema(dml).map(drop).unwrap_err());
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

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: Invalid referential action: `Restrict`. Allowed values: (`Cascade`, `NoAction`, `SetNull`, `SetDefault`)[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m    aId Int
        [1;94m14 | [0m    a A @relation(fields: [aId], references: [id], onUpdate: Restrict, [1;91monDelete: Restrict[0m)
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: Invalid referential action: `Restrict`. Allowed values: (`Cascade`, `NoAction`, `SetNull`, `SetDefault`)[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m    aId Int
        [1;94m14 | [0m    a A @relation(fields: [aId], references: [id], [1;91monUpdate: Restrict[0m, onDelete: Restrict)
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&datamodel::parse_schema(dml).map(drop).unwrap_err());
}

#[test]
fn actions_should_be_defined_only_from_one_side() {
    let dml = indoc! { r#"
        datasource db {
            provider = "sqlserver"
            url = "sqlserver://"
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

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation fields `b` on Model `A` and `a` on Model `B` both provide the `onDelete` or `onUpdate` argument in the @relation attribute. You have to provide it only on one of the two fields.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m    id Int @id
        [1;94m 8 | [0m    [1;91mb B? @relation(onUpdate: NoAction, onDelete: NoAction)[0m
        [1;94m 9 | [0m}
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation fields `a` on Model `B` and `b` on Model `A` both provide the `onDelete` or `onUpdate` argument in the @relation attribute. You have to provide it only on one of the two fields.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m    aId Int
        [1;94m14 | [0m    [1;91ma A @relation(fields: [aId], references: [id], onUpdate: NoAction, onDelete: NoAction)[0m
        [1;94m15 | [0m}
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&datamodel::parse_schema(dml).map(drop).unwrap_err());
}

#[test]
fn set_default_action_should_not_work_on_prisma_level_referential_integrity() {
    let dml = indoc!(
        r#"
            datasource db {
                provider = "mysql"
                referentialIntegrity = "prisma"
                url = "mysql://root:prisma@localhost:3306/mydb"
            }

            generator client {
                provider = "prisma-client-js"
                previewFeatures = ["referentialIntegrity"]
            }

            model A {{
                id Int @id @map("_id")
                bs B[]
            }

            model B {
                id Int @id @map("_id")
                aId Int
                a A @relation(fields: [aId], references: [id], onDelete: SetDefault)
            }
        "#,
    );

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: Invalid referential action: `SetDefault`. Allowed values: (`Cascade`, `Restrict`, `NoAction`, `SetNull`)[0m
          [1;94m-->[0m  [4mschema.prisma:20[0m
        [1;94m   | [0m
        [1;94m19 | [0m    aId Int
        [1;94m20 | [0m    a A @relation(fields: [aId], references: [id], [1;91monDelete: SetDefault[0m)
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&datamodel::parse_schema(dml).map(drop).unwrap_err());
}

#[test]
fn on_delete_cannot_be_defined_on_the_wrong_side_1_n() {
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

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation field `bs` on Model `A` must not specify the `onDelete` or `onUpdate` argument in the @relation attribute. You must only specify it on the opposite field `a` on model `B`, or in case of a many to many relation, in an explicit join table.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m    id Int @id
        [1;94m 8 | [0m    [1;91mbs B[] @relation(onDelete: Restrict)[0m
        [1;94m 9 | [0m}
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&datamodel::parse_schema(dml).map(drop).unwrap_err());
}

#[test]
fn on_update_cannot_be_defined_on_the_wrong_side_1_n() {
    let dml = indoc! { r#"
        datasource db {
            provider = "mysql"
            url = "mysql://"
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

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation field `bs` on Model `A` must not specify the `onDelete` or `onUpdate` argument in the @relation attribute. You must only specify it on the opposite field `a` on model `B`, or in case of a many to many relation, in an explicit join table.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m    id Int @id
        [1;94m 8 | [0m    [1;91mbs B[] @relation(onUpdate: Restrict)[0m
        [1;94m 9 | [0m}
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&datamodel::parse_schema(dml).map(drop).unwrap_err());
}

#[test]
fn on_delete_cannot_be_defined_on_the_wrong_side_1_1() {
    let dml = indoc! { r#"
        datasource db {
            provider = "mysql"
            url = "mysql://"
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
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m    id        Int      @id @default(autoincrement())
        [1;94m 8 | [0m    [1;91mcock      Chicken? @relation(name: "a_self_relation", onDelete: NoAction)[0m
        [1;94m 9 | [0m    hen       Chicken? @relation(name: "a_self_relation", fields: [chickenId], references: [id])
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

        model Chicken {
            id        Int      @id @default(autoincrement())
            cock      Chicken? @relation(name: "a_self_relation", onUpdate: NoAction)
            hen       Chicken? @relation(name: "a_self_relation", fields: [chickenId], references: [id])
            chickenId Int?
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation field `cock` on Model `Chicken` must not specify the `onDelete` or `onUpdate` argument in the @relation attribute. You must only specify it on the opposite field `hen` on model `Chicken`.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m    id        Int      @id @default(autoincrement())
        [1;94m 8 | [0m    [1;91mcock      Chicken? @relation(name: "a_self_relation", onUpdate: NoAction)[0m
        [1;94m 9 | [0m    hen       Chicken? @relation(name: "a_self_relation", fields: [chickenId], references: [id])
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&datamodel::parse_schema(dml).map(drop).unwrap_err());
}
