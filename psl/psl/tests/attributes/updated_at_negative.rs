use crate::common::*;

#[test]
fn should_fail_if_field_type_is_string() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          lastSeen String @updatedAt
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@updatedAt": Fields that are marked with @updatedAt must be of type DateTime.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  lastSeen String [1;91m@updatedAt[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn should_fail_if_field_arity_is_list() {
    let dml = indoc! {r#"
        datasource db {
          provider = "postgres"
          url = "postgres://"
        }

        model User {
          id Int @id
          lastSeen DateTime[] @updatedAt
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@updatedAt": Fields that are marked with @updatedAt cannot be lists.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  id Int @id
        [1;94m 8 | [0m  lastSeen DateTime[] [1;91m@updatedAt[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}
