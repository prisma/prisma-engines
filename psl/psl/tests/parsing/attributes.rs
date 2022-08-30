use crate::common::*;

#[test]
fn empty_arguments_are_rejected_with_nice_error() {
    let schema = r#"
        model A {
            id Int @id
            bs B[]
        }

        model B {
            id Int @id
            aId Int
            a   A @relation(fields: [aId], onDelete: , references: [id])
        }
    "#;

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": The `onDelete` argument is missing a value.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m            aId Int
        [1;94m10 | [0m            a   A @relation(fields: [aId], [1;91monDelete[0m: , references: [id])
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expected)
}

#[test]
fn empty_model_attribute_arguments_are_rejected_with_nice_error() {
    let schema = r#"
        model B {
            id Int

            @@id([id], name: )
        }
    "#;

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@id": The `name` argument is missing a value.[0m
          [1;94m-->[0m  [4mschema.prisma:5[0m
        [1;94m   | [0m
        [1;94m 4 | [0m
        [1;94m 5 | [0m            @@id([id], [1;91mname[0m: )
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expected)
}

#[test]
fn empty_enum_attribute_arguments_are_rejected_with_nice_error() {
    let schema = r#"
        enum Colour {
            RED
            GREEN
            BLUE

            @@map(name:)
        }
    "#;

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@map": The `name` argument is missing a value.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m
        [1;94m 7 | [0m            @@map([1;91mname[0m:)
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expected)
}

#[test]
fn trailing_commas_without_space_are_rejected_with_nice_error() {
    let schema = r#"
        enum Colour {
            RED
            GREEN
            BLUE

            @@map(name: "color",)
        }
    "#;

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@map": Trailing commas are not valid in attribute arguments, please remove the comma.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m
        [1;94m 7 | [0m            @@map(name: "color"[1;91m,[0m)
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expected)
}

#[test]
fn trailing_commas_with_space_are_rejected_with_nice_error() {
    let schema = r#"
        enum Colour {
            RED
            GREEN
            BLUE

            @@map(name: "color", )
        }
    "#;

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@map": Trailing commas are not valid in attribute arguments, please remove the comma.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m
        [1;94m 7 | [0m            @@map(name: "color"[1;91m,[0m )
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expected)
}
