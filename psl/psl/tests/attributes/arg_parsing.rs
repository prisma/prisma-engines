use crate::common::*;

#[test]
fn fail_on_duplicate_attribute() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          firstName String @map(name: "first_name", name: "Duplicate")
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mArgument "name" is already specified.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  firstName String @map(name: "first_name", [1;91mname: "Duplicate"[0m)
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn fail_on_duplicate_unnamed_attribute() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          firstName String @map("first_name", name: "Duplicate")
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mArgument "name" is already specified as unnamed argument.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  firstName String @map("first_name", [1;91mname: "Duplicate"[0m)
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn fail_on_extra_argument() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          firstName String @map("first_name", unused: "Unnamed")
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mNo such argument.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  firstName String @map("first_name", [1;91munused: "Unnamed"[0m)
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}
