use crate::common::*;

#[test]
fn trailing_commas_in_function_arguments_list() {
    let input = indoc!(
        r#"
        model Category {
          id Int @id @default(dbgenerated("newId()",))
        }"#
    );

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": Trailing commas are not valid in attribute arguments, please remove the comma.[0m
          [1;94m-->[0m  [4mschema.prisma:2[0m
        [1;94m   | [0m
        [1;94m 1 | [0mmodel Category {
        [1;94m 2 | [0m  id Int @id @default(dbgenerated("newId()"[1;91m,[0m))
        [1;94m   | [0m
    "#]];

    expect_error(input, &expected);
}

#[test]
fn empty_arguments_in_function_arguments_list() {
    let input = indoc!(
        r#"
        model Category {
          id Int @id @default(dbgenerated(name: ))
        }"#
    );

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@default": The `name` argument is missing a value.[0m
          [1;94m-->[0m  [4mschema.prisma:2[0m
        [1;94m   | [0m
        [1;94m 1 | [0mmodel Category {
        [1;94m 2 | [0m  id Int @id @default(dbgenerated([1;91mname[0m: ))
        [1;94m   | [0m
    "#]];

    expect_error(input, &expected);
}

#[test]
fn empty_arguments_in_index_fields() {
    let input = indoc! {r#"
        model A {
          id  Int   @id
          val Int[]

          @@index([val(ops: )], type: Gin)
        }
    "#};

    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The `ops` argument is missing a value.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  @@index([val([1;91mops[0m: )], type: Gin)
        [1;94m   | [0m
    "#]];

    let input = crate::with_header(input, crate::Provider::Postgres, &[]);

    expect_error(&input, &expected);
}
