use crate::common::*;

#[test]
fn cascading_on_delete_self_relations() {
    let dml = indoc! {
        r#"
        datasource db {
            provider = "sqlserver"
            url = "sqlserver://"
        }

        model A {
            id     Int  @id @default(autoincrement())
            child  A?   @relation(name: "a_self_relation")
            parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onDelete: Cascade)
            aId    Int? @unique
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating: A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes. (Implicit default `onUpdate`: `Cascade`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m    child  A?   @relation(name: "a_self_relation")
        [1;94m 9 | [0m    [1;91mparent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onDelete: Cascade)[0m
        [1;94m10 | [0m    aId    Int? @unique
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn cascading_cycles_cannot_loop_infinitely() {
    let dml = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url = "sqlserver://"
        }

        model A {
          id Int @id
          name Int @unique
          c C @relation(name: "atoc", fields: [name], references: [name], onDelete: Cascade)
          cs C[] @relation(name: "ctoa")
        }

        model B {
          id Int @id
          name Int @unique
          c C @relation(name: "btoc", fields: [name], references: [name], onDelete: Cascade)
        }

        model C {
          id Int @id
          name Int @unique
          a A @relation(name: "ctoa", fields: [name], references: [name], onDelete: Cascade)
          as A[] @relation(name: "atoc")
          bs B[] @relation(name: "btoc")
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating: Reference causes a cycle. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: A.c → C.a. (Implicit default `onUpdate`: `Cascade`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  name Int @unique
        [1;94m 9 | [0m  [1;91mc C @relation(name: "atoc", fields: [name], references: [name], onDelete: Cascade)[0m
        [1;94m10 | [0m  cs C[] @relation(name: "ctoa")
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: Reference causes a cycle. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: C.a → A.c. (Implicit default `onUpdate`: `Cascade`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:22[0m
        [1;94m   | [0m
        [1;94m21 | [0m  name Int @unique
        [1;94m22 | [0m  [1;91ma A @relation(name: "ctoa", fields: [name], references: [name], onDelete: Cascade)[0m
        [1;94m23 | [0m  as A[] @relation(name: "atoc")
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn cycles_are_allowed_outside_of_emulation_and_sqlserver() {
    let dml = indoc! {
        r#"
        datasource db {
            provider = "mysql"
            url = "mysql://"
        }

        generator js1 {
          provider = "javascript"
        }

        model A {
            id     Int  @id @default(autoincrement())
            child  A?   @relation(name: "a_self_relation")
            parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onDelete: Cascade)
            aId    Int? @unique
        }
    "#};

    assert_valid(dml)
}

#[test]
fn emulated_cascading_on_delete_self_relations() {
    let dml = indoc! {
        r#"
        datasource db {
            provider = "mysql"
            url = "mysql://"
            relationMode = "prisma"
        }

        generator js1 {
          provider = "javascript"
        }

        model A {
            id     Int  @id @default(autoincrement())
            child  A?   @relation(name: "a_self_relation")
            parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onDelete: Cascade)
            aId    Int? @unique
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating: A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes. (Implicit default `onUpdate`: `Cascade`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m    child  A?   @relation(name: "a_self_relation")
        [1;94m14 | [0m    [1;91mparent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onDelete: Cascade)[0m
        [1;94m15 | [0m    aId    Int? @unique
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn cascading_on_update_self_relations() {
    let dml = indoc! {
        r#"
        datasource db {
            provider = "sqlserver"
            url = "sqlserver://"
        }

        model A {
            id     Int  @id @default(autoincrement())
            child  A?   @relation(name: "a_self_relation")
            parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onUpdate: Cascade)
            aId    Int? @unique
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating: A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes. (Implicit default `onDelete`: `SetNull`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m    child  A?   @relation(name: "a_self_relation")
        [1;94m 9 | [0m    [1;91mparent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onUpdate: Cascade)[0m
        [1;94m10 | [0m    aId    Int? @unique
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn emulated_cascading_on_update_self_relations() {
    let dml = indoc! {
        r#"
        datasource db {
            provider = "mysql"
            url = "mysql://"
            relationMode = "prisma"
        }

        generator js1 {
          provider = "javascript"
        }

        model A {
            id     Int  @id @default(autoincrement())
            child  A?   @relation(name: "a_self_relation")
            parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onUpdate: Cascade)
            aId    Int? @unique
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating: A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes. (Implicit default `onDelete`: `SetNull`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m    child  A?   @relation(name: "a_self_relation")
        [1;94m14 | [0m    [1;91mparent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onUpdate: Cascade)[0m
        [1;94m15 | [0m    aId    Int? @unique
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn null_setting_on_delete_self_relations() {
    let dml = indoc! {
        r#"
        datasource db {
            provider = "sqlserver"
            url = "sqlserver://"
        }

        model A {
            id     Int  @id @default(autoincrement())
            child  A?   @relation(name: "a_self_relation")
            parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onDelete: SetNull)
            aId    Int? @unique
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating: A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes. (Implicit default `onUpdate`: `Cascade`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m    child  A?   @relation(name: "a_self_relation")
        [1;94m 9 | [0m    [1;91mparent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onDelete: SetNull)[0m
        [1;94m10 | [0m    aId    Int? @unique
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn emulated_null_setting_on_delete_self_relations() {
    let dml = indoc! {
        r#"
        datasource db {
            provider = "mysql"
            url = "mysql://"
            relationMode = "prisma"
        }

        generator js1 {
          provider = "javascript"
        }

        model A {
            id     Int  @id @default(autoincrement())
            child  A?   @relation(name: "a_self_relation")
            parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onDelete: SetNull)
            aId    Int? @unique
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating: A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes. (Implicit default `onUpdate`: `Cascade`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m    child  A?   @relation(name: "a_self_relation")
        [1;94m14 | [0m    [1;91mparent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onDelete: SetNull)[0m
        [1;94m15 | [0m    aId    Int? @unique
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn null_setting_on_update_self_relations() {
    let dml = indoc! {
        r#"
        datasource db {
            provider = "sqlserver"
            url = "sqlserver://"
        }

        model A {
            id     Int  @id @default(autoincrement())
            child  A?   @relation(name: "a_self_relation")
            parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onUpdate: SetNull)
            aId    Int? @unique
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating: A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes. (Implicit default `onDelete`: `SetNull`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m    child  A?   @relation(name: "a_self_relation")
        [1;94m 9 | [0m    [1;91mparent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onUpdate: SetNull)[0m
        [1;94m10 | [0m    aId    Int? @unique
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn emulated_null_setting_on_update_self_relations() {
    let dml = indoc! {
        r#"
        datasource db {
            provider = "mysql"
            url = "mysql://"
            relationMode = "prisma"
        }

        generator js1 {
          provider = "javascript"
        }

        model A {
            id     Int  @id @default(autoincrement())
            child  A?   @relation(name: "a_self_relation")
            parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onUpdate: SetNull)
            aId    Int? @unique
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating: A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes. (Implicit default `onDelete`: `SetNull`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m    child  A?   @relation(name: "a_self_relation")
        [1;94m14 | [0m    [1;91mparent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onUpdate: SetNull)[0m
        [1;94m15 | [0m    aId    Int? @unique
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn default_setting_on_delete_self_relations() {
    let dml = indoc! {
        r#"
        datasource db {
            provider = "sqlserver"
            url = "sqlserver://"
        }

        model A {
            id     Int  @id @default(autoincrement())
            child  A?   @relation(name: "a_self_relation")
            parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onDelete: SetDefault)
            aId    Int? @unique
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating: A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes. (Implicit default `onUpdate`: `Cascade`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m    child  A?   @relation(name: "a_self_relation")
        [1;94m 9 | [0m    [1;91mparent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onDelete: SetDefault)[0m
        [1;94m10 | [0m    aId    Int? @unique
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn emulated_default_setting_on_delete_self_relations() {
    let dml = indoc! {
        r#"
        datasource db {
            provider = "mysql"
            url = "mysql://"
            relationMode = "prisma"
        }

        generator js1 {
          provider = "javascript"
        }

        model A {
            id     Int  @id @default(autoincrement())
            child  A?   @relation(name: "a_self_relation")
            parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onDelete: SetDefault)
            aId    Int? @unique
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating: Invalid referential action: `SetDefault`. Allowed values: (`Cascade`, `Restrict`, `NoAction`, `SetNull`)[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m    child  A?   @relation(name: "a_self_relation")
        [1;94m14 | [0m    parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], [1;91monDelete: SetDefault[0m)
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes. (Implicit default `onUpdate`: `Cascade`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m    child  A?   @relation(name: "a_self_relation")
        [1;94m14 | [0m    [1;91mparent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onDelete: SetDefault)[0m
        [1;94m15 | [0m    aId    Int? @unique
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn default_setting_on_update_self_relations() {
    let dml = indoc! {
        r#"
        datasource db {
            provider = "sqlserver"
            url = "sqlserver://"
        }

        model A {
            id     Int  @id @default(autoincrement())
            child  A?   @relation(name: "a_self_relation")
            parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onUpdate: SetDefault)
            aId    Int? @unique
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating: A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes. (Implicit default `onDelete`: `SetNull`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m    child  A?   @relation(name: "a_self_relation")
        [1;94m 9 | [0m    [1;91mparent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onUpdate: SetDefault)[0m
        [1;94m10 | [0m    aId    Int? @unique
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn emulated_default_setting_on_update_self_relations() {
    let dml = indoc! {
        r#"
        datasource db {
            provider = "mysql"
            url = "mysql://"
            relationMode = "prisma"
        }

        generator js1 {
          provider = "javascript"
        }

        model A {
            id     Int  @id @default(autoincrement())
            child  A?   @relation(name: "a_self_relation")
            parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onUpdate: SetDefault)
            aId    Int? @unique
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating: Invalid referential action: `SetDefault`. Allowed values: (`Cascade`, `Restrict`, `NoAction`, `SetNull`)[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m    child  A?   @relation(name: "a_self_relation")
        [1;94m14 | [0m    parent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], [1;91monUpdate: SetDefault[0m)
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: A self-relation must have `onDelete` and `onUpdate` referential actions set to `NoAction` in one of the @relation attributes. (Implicit default `onDelete`: `SetNull`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m    child  A?   @relation(name: "a_self_relation")
        [1;94m14 | [0m    [1;91mparent A?   @relation(name: "a_self_relation", fields: [aId], references: [id], onUpdate: SetDefault)[0m
        [1;94m15 | [0m    aId    Int? @unique
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn cascading_cyclic_one_hop_relations() {
    let dml = indoc! {
        r#"
        datasource db {
            provider = "sqlserver"
            url = "sqlserver://"
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
        [1;91merror[0m: [1mError validating: Reference causes a cycle. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: A.b → B.a. (Implicit default `onUpdate`: `Cascade`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m    id     Int  @id @default(autoincrement())
        [1;94m 8 | [0m    [1;91mb      B    @relation(name: "foo", fields: [bId], references: [id], onDelete: Cascade)[0m
        [1;94m 9 | [0m    bId    Int
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: Reference causes a cycle. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: B.a → A.b. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m    id     Int @id @default(autoincrement())
        [1;94m15 | [0m    [1;91ma      A   @relation(name: "bar", fields: [aId], references: [id], onUpdate: Cascade)[0m
        [1;94m16 | [0m    as     A[] @relation(name: "foo")
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn emulated_cascading_cyclic_one_hop_relations() {
    let dml = indoc! {
        r#"
        datasource db {
            provider = "mysql"
            url = "mysql://"
            relationMode = "prisma"
        }

        generator js1 {
          provider = "javascript"
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
        [1;91merror[0m: [1mError validating: Reference causes a cycle. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: A.b → B.a. (Implicit default `onUpdate`: `Cascade`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m    id     Int  @id @default(autoincrement())
        [1;94m13 | [0m    [1;91mb      B    @relation(name: "foo", fields: [bId], references: [id], onDelete: Cascade)[0m
        [1;94m14 | [0m    bId    Int
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: Reference causes a cycle. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: B.a → A.b. Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:20[0m
        [1;94m   | [0m
        [1;94m19 | [0m    id     Int @id @default(autoincrement())
        [1;94m20 | [0m    [1;91ma      A   @relation(name: "bar", fields: [aId], references: [id], onUpdate: Cascade)[0m
        [1;94m21 | [0m    as     A[] @relation(name: "foo")
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn cascading_cyclic_hop_over_table_relations() {
    let dml = indoc! {
        r#"
        datasource db {
            provider = "sqlserver"
            url = "sqlserver://"
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
        [1;91merror[0m: [1mError validating: Reference causes a cycle. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: A.b → B.c → C.a. (Implicit default `onUpdate`: `Cascade`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m    bId    Int
        [1;94m 9 | [0m    [1;91mb      B    @relation(fields: [bId], references: [id])[0m
        [1;94m10 | [0m    cs     C[]
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: Reference causes a cycle. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: B.c → C.a → A.b. (Implicit default `onUpdate`: `Cascade`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m    cId    Int
        [1;94m17 | [0m    [1;91mc      C    @relation(fields: [cId], references: [id])[0m
        [1;94m18 | [0m}
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: Reference causes a cycle. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: C.a → A.b → B.c. (Implicit default `onUpdate`: `Cascade`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:24[0m
        [1;94m   | [0m
        [1;94m23 | [0m    aId    Int
        [1;94m24 | [0m    [1;91ma      A   @relation(fields: [aId], references: [id])[0m
        [1;94m25 | [0m}
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn multiple_cascading_simple() {
    let dml = indoc! {
        r#"
        datasource test {
            provider = "sqlserver"
            url      = "sqlserver://localhost:1433;database=master;user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true"
        }

        model User {
            id        Int       @id @default(autoincrement())
            addressId Int
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
        [1;91merror[0m: [1mError validating: When any of the records in model `User` is updated or deleted, the referential actions on the relations cascade to model `Comment` through multiple paths. Please break one of these paths by setting the `onUpdate` and `onDelete` to `NoAction`. (Implicit default `onUpdate`: `Cascade`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:25[0m
        [1;94m   | [0m
        [1;94m24 | [0m    postId      Int
        [1;94m25 | [0m    [1;91mwrittenBy   User     @relation(fields: [writtenById], references: [id])[0m
        [1;94m26 | [0m    post        Post     @relation(fields: [postId], references: [id])
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: When any of the records in model `User` is updated or deleted, the referential actions on the relations cascade to model `Comment` through multiple paths. Please break one of these paths by setting the `onUpdate` and `onDelete` to `NoAction`. (Implicit default `onUpdate`: `Cascade`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:26[0m
        [1;94m   | [0m
        [1;94m25 | [0m    writtenBy   User     @relation(fields: [writtenById], references: [id])
        [1;94m26 | [0m    [1;91mpost        Post     @relation(fields: [postId], references: [id])[0m
        [1;94m27 | [0m}
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn multiple_cascading_complex() {
    let dml = indoc! {
        r#"
        datasource test {
            provider = "sqlserver"
            url      = "sqlserver://localhost:1433;database=master;user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true"
        }

        model Address {
            id        Int       @id @default(autoincrement())
            users     User[]
        }

        model User {
            id        Int       @id @default(autoincrement())
            addressId Int
            address   Address   @relation(fields: [addressId], references: [id])
            comments  Comment[]
            posts     Post[]
            cements   Cement[]
        }

        model Post {
            id        Int       @id @default(autoincrement())
            authorId  Int
            author    User      @relation(fields: [authorId], references: [id])
            comments  Comment[]
            tags      Tag[]     @relation("TagToPost")
            cements   Cement[]
        }

        model Cement {
            id          Int       @id @default(autoincrement())
            postId      Int
            userId      Int
            tagId       Int
            post        Post      @relation(fields: [postId], references: [id])
            user        User      @relation(fields: [userId], references: [id])
            tag         Tag       @relation(fields: [tagId], references: [id])
            comments    Comment[]
        }

        model Comment {
            id          Int      @id @default(autoincrement())
            writtenById Int
            postId      Int
            cementId    Int
            writtenBy   User     @relation(fields: [writtenById], references: [id])
            post        Post     @relation(fields: [postId], references: [id])
            cement      Cement   @relation(fields: [cementId], references: [id])
        }

        model Tag {
            id      Int    @id @default(autoincrement())
            tag     String @unique
            posts   Post[] @relation("TagToPost")
            cements Cement[] 
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating: When any of the records in models `Address`, `User` are updated or deleted, the referential actions on the relations cascade to model `Cement` through multiple paths. Please break one of these paths by setting the `onUpdate` and `onDelete` to `NoAction`. (Implicit default `onUpdate`: `Cascade`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:34[0m
        [1;94m   | [0m
        [1;94m33 | [0m    tagId       Int
        [1;94m34 | [0m    [1;91mpost        Post      @relation(fields: [postId], references: [id])[0m
        [1;94m35 | [0m    user        User      @relation(fields: [userId], references: [id])
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: When any of the records in models `Address`, `User` are updated or deleted, the referential actions on the relations cascade to model `Cement` through multiple paths. Please break one of these paths by setting the `onUpdate` and `onDelete` to `NoAction`. (Implicit default `onUpdate`: `Cascade`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:35[0m
        [1;94m   | [0m
        [1;94m34 | [0m    post        Post      @relation(fields: [postId], references: [id])
        [1;94m35 | [0m    [1;91muser        User      @relation(fields: [userId], references: [id])[0m
        [1;94m36 | [0m    tag         Tag       @relation(fields: [tagId], references: [id])
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: When any of the records in models `Address`, `User` are updated or deleted, the referential actions on the relations cascade to model `Comment` through multiple paths. Please break one of these paths by setting the `onUpdate` and `onDelete` to `NoAction`. (Implicit default `onUpdate`: `Cascade`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:45[0m
        [1;94m   | [0m
        [1;94m44 | [0m    cementId    Int
        [1;94m45 | [0m    [1;91mwrittenBy   User     @relation(fields: [writtenById], references: [id])[0m
        [1;94m46 | [0m    post        Post     @relation(fields: [postId], references: [id])
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: When any of the records in models `Address`, `Post`, `User` are updated or deleted, the referential actions on the relations cascade to model `Comment` through multiple paths. Please break one of these paths by setting the `onUpdate` and `onDelete` to `NoAction`. (Implicit default `onUpdate`: `Cascade`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:46[0m
        [1;94m   | [0m
        [1;94m45 | [0m    writtenBy   User     @relation(fields: [writtenById], references: [id])
        [1;94m46 | [0m    [1;91mpost        Post     @relation(fields: [postId], references: [id])[0m
        [1;94m47 | [0m    cement      Cement   @relation(fields: [cementId], references: [id])
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: When any of the records in models `Address`, `Post`, `User` are updated or deleted, the referential actions on the relations cascade to model `Comment` through multiple paths. Please break one of these paths by setting the `onUpdate` and `onDelete` to `NoAction`. (Implicit default `onUpdate`: `Cascade`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:47[0m
        [1;94m   | [0m
        [1;94m46 | [0m    post        Post     @relation(fields: [postId], references: [id])
        [1;94m47 | [0m    [1;91mcement      Cement   @relation(fields: [cementId], references: [id])[0m
        [1;94m48 | [0m}
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn cycle_detection_prints_the_right_path() {
    let dm = r#"
    datasource db {
        provider = "sqlserver"
        url = "sqlserver://"
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
        [1;91merror[0m: [1mError validating: Reference causes a cycle. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: Post.user → User.comment → Comment.post. (Implicit default `onUpdate`: `Cascade`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m        user_id  Int       @map("bId")
        [1;94m10 | [0m        [1;91muser     User      @relation(fields: [user_id], references: [id])[0m
        [1;94m11 | [0m        comments Comment[]
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: Reference causes a cycle. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: User.comment → Comment.post → Post.user. (Implicit default `onUpdate`: `Cascade`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:21[0m
        [1;94m   | [0m
        [1;94m20 | [0m        address    Address @relation(fields: [address_id], references: [id])
        [1;94m21 | [0m        [1;91mcomment    Comment @relation(fields: [comment_id], references: [id])[0m
        [1;94m22 | [0m        @@map("B")
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: Reference causes a cycle. One of the @relation attributes in this cycle must have `onDelete` and `onUpdate` referential actions set to `NoAction`. Cycle path: Comment.post → Post.user → User.comment. (Implicit default `onUpdate`: `Cascade`) Read more at https://pris.ly/d/cyclic-referential-actions[0m
          [1;94m-->[0m  [4mschema.prisma:48[0m
        [1;94m   | [0m
        [1;94m47 | [0m        post_id Int    @map("aId")
        [1;94m48 | [0m        [1;91mpost    Post   @relation(fields: [post_id], references: [id])[0m
        [1;94m49 | [0m        @@map("C")
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dm));
}

#[test]
fn separate_non_crossing_cascade_paths_should_work() {
    let dm = r#"
        datasource db {
            provider = "sqlserver"
            url      = "sqlserver://localhost:1433;database=master;user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true"
        }

        model order_items {
            order_id   Int
            item_id    Int
            product_id Int
            orders     orders   @relation(fields: [order_id], references: [order_id], onDelete: Cascade)
            products   products @relation(fields: [product_id], references: [product_id], onDelete: Cascade)

            @@id([order_id, item_id])
        }

        model orders {
            order_id      Int           @id @default(autoincrement())
            store_id      Int
            staff_id      Int
            staffs        staffs        @relation(fields: [staff_id], references: [staff_id], onUpdate: NoAction)
            stores        stores        @relation(fields: [store_id], references: [store_id], onDelete: Cascade)
            order_items   order_items[]
        }

        model products {
            product_id   Int           @id @default(autoincrement())
            brand_id     Int
            order_items  order_items[]
            stocks       stocks[]
        }

        model staffs {
            staff_id     Int      @id @default(autoincrement())
            store_id     Int
            manager_id   Int?
            staffs       staffs?  @relation("staffsTostaffs_manager_id", fields: [manager_id], references: [staff_id], onDelete: NoAction, onUpdate: NoAction)
            stores       stores   @relation(fields: [store_id], references: [store_id], onDelete: Cascade)
            orders       orders[]
            other_staffs staffs[] @relation("staffsTostaffs_manager_id")
        }

        model stocks {
            store_id   Int
            product_id Int
            products   products @relation(fields: [product_id], references: [product_id], onDelete: Cascade)
            stores     stores   @relation(fields: [store_id], references: [store_id], onDelete: Cascade)

            @@id([store_id, product_id])
        }

        model stores {
            store_id   Int      @id @default(autoincrement())
            orders     orders[]
            staffs     staffs[]
            stocks     stocks[]
        }
    "#;

    assert_valid(dm)
}
