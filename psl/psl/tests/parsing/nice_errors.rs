use crate::common::*;

#[test]
fn nice_error_for_missing_model_keyword() {
    let dml = indoc! {r#"
        User {
          id Int @id
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating: This block is invalid. It does not start with any known Prisma schema keyword. Valid keywords include 'model', 'enum', 'type', 'datasource' and 'generator'.[0m
          [1;94m-->[0m  [4mschema.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0m[1;91mUser {[0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m}
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation)
}

#[test]
fn nice_error_for_missing_model_keyword_2() {
    let dml = indoc! {r#"
        model User {
          id Int @id
        }

        Todo {
          id
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating: This block is invalid. It does not start with any known Prisma schema keyword. Valid keywords include 'model', 'enum', 'type', 'datasource' and 'generator'.[0m
          [1;94m-->[0m  [4mschema.prisma:5[0m
        [1;94m   | [0m
        [1;94m 4 | [0m
        [1;94m 5 | [0m[1;91mTodo {[0m
        [1;94m 6 | [0m  id
        [1;94m 7 | [0m}
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation)
}

#[test]
fn nice_error_on_incorrect_enum_field() {
    let dml = indoc! {r#"
        enum Role {
          A-dmin
          User
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating: The character `-` is not allowed in Enum Value names.[0m
          [1;94m-->[0m  [4mschema.prisma:2[0m
        [1;94m   | [0m
        [1;94m 1 | [0menum Role {
        [1;94m 2 | [0m  [1;91mA-dmin[0m
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation)
}

#[test]
fn nice_error_missing_type() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          name
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": This field declaration is invalid. It is either missing a name or a type.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  [1;91mname[0m
        [1;94m 4 | [0m}
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation)
}

#[test]
fn nice_error_missing_attribute_name() {
    let dml = indoc! {r#"
        model User {
          id Int @id @
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating: This line is not a valid field or attribute definition.[0m
          [1;94m-->[0m  [4mschema.prisma:2[0m
        [1;94m   | [0m
        [1;94m 1 | [0mmodel User {
        [1;94m 2 | [0m  [1;91mid Int @id @[0m
        [1;94m 3 | [0m}
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation)
}

#[test]
fn nice_error_missing_braces() {
    let dml = indoc! {r#"
        model User
          id Int @id
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating: This line is invalid. It does not start with any known Prisma schema keyword.[0m
          [1;94m-->[0m  [4mschema.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0m[1;91mmodel User[0m
        [1;94m 2 | [0m  id Int @id
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: This line is invalid. It does not start with any known Prisma schema keyword.[0m
          [1;94m-->[0m  [4mschema.prisma:2[0m
        [1;94m   | [0m
        [1;94m 1 | [0mmodel User
        [1;94m 2 | [0m  [1;91mid Int @id[0m
        [1;94m 3 | [0m
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation)
}

#[test]
fn nice_error_broken_field_type_legacy_list() {
    let dml = indoc! {r#"
        model User {
          id [Int] @id
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mTo specify a list, please use `Type[]` instead of `[Type]`.[0m
          [1;94m-->[0m  [4mschema.prisma:2[0m
        [1;94m   | [0m
        [1;94m 1 | [0mmodel User {
        [1;94m 2 | [0m  id [1;91m[Int][0m @id
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation)
}

#[test]
fn nice_error_broken_field_type_legacy_colon() {
    let dml = indoc! {r#"
        model User {
          id: Int @id
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mField declarations don't require a `:`.[0m
          [1;94m-->[0m  [4mschema.prisma:2[0m
        [1;94m   | [0m
        [1;94m 1 | [0mmodel User {
        [1;94m 2 | [0m  id[1;91m:[0m Int @id
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation)
}

#[test]
fn nice_error_broken_field_type_legacy_required() {
    let dml = indoc! {r#"
        model User {
          id Int! @id
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mFields are required by default, `!` is no longer required.[0m
          [1;94m-->[0m  [4mschema.prisma:2[0m
        [1;94m   | [0m
        [1;94m 1 | [0mmodel User {
        [1;94m 2 | [0m  id [1;91mInt![0m @id
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation)
}

#[test]
fn nice_error_in_case_of_literal_type_in_env_var() {
    let source = indoc! {r#"
        datasource ds {
          provider = "postgresql"
          url = env(DATABASE_URL)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mExpected a string value, but received literal value `DATABASE_URL`.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "postgresql"
        [1;94m 3 | [0m  url = env([1;91mDATABASE_URL[0m)
        [1;94m   | [0m
    "#]];

    expect_error(source, &expectation)
}

#[test]
fn nice_error_in_case_of_bool_type_in_env_var() {
    let source = indoc! {r#"
        datasource ds {
          provider = "postgresql"
          url = env(true)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mExpected a string value, but received literal value `true`.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "postgresql"
        [1;94m 3 | [0m  url = env([1;91mtrue[0m)
        [1;94m   | [0m
    "#]];

    expect_error(source, &expectation)
}

#[test]
fn nice_error_in_case_of_numeric_type_in_env_var() {
    let source = indoc! {r#"
        datasource ds {
          provider = "postgresql"
          url = env(4)
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mExpected a string value, but received numeric value `4`.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "postgresql"
        [1;94m 3 | [0m  url = env([1;91m4[0m)
        [1;94m   | [0m
    "#]];

    expect_error(source, &expectation)
}

#[test]
fn nice_error_in_case_of_array_type_in_env_var() {
    let source = indoc! {r#"
        datasource ds {
          provider = "postgresql"
          url = env([DATABASE_URL])
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mExpected a string value, but received array value `[DATABASE_URL]`.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "postgresql"
        [1;94m 3 | [0m  url = env([1;91m[DATABASE_URL][0m)
        [1;94m   | [0m
    "#]];

    expect_error(source, &expectation)
}

#[test]
fn optional_list_fields_must_error() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          names String[]?
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mOptional lists are not supported. Use either `Type[]` or `Type?`.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  names [1;91mString[]?[0m
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation)
}

#[test]
fn invalid_lines_at_the_top_level_must_render_nicely() {
    // https://github.com/prisma/vscode/issues/140
    // If a user types on the very last line we did not error nicely.
    // a new line fixed the problem but this is not nice.
    let dml = indoc! {r#"
        model User {
          id Int @id
          names String
        }

        model Bl
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating: This line is invalid. It does not start with any known Prisma schema keyword.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m
        [1;94m 6 | [0m[1;91mmodel Bl[0m
        [1;94m 7 | [0m
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation)
}

#[test]
fn invalid_lines_in_datasources_must_render_nicely() {
    let dml = indoc! {r#"
        datasource mydb {
          provider = "postgres"
          url = "postgresql://localhost"
          this is an invalid line
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating: This line is not a valid definition within a datasource.[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m  url = "postgresql://localhost"
        [1;94m 4 | [0m  [1;91mthis is an invalid line[0m
        [1;94m 5 | [0m}
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation)
}

#[test]
fn invalid_lines_in_generators_must_render_nicely() {
    let dml = indoc! {r#"
        generator js {
          provider = "js"
          this is an invalid line
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating: This line is not a valid definition within a generator.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "js"
        [1;94m 3 | [0m  [1;91mthis is an invalid line[0m
        [1;94m 4 | [0m}
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation)
}

#[test]
fn invalid_field_line_must_error_nicely() {
    // https://github.com/prisma/vscode/issues/140
    // If a user types on the very last line we did not error nicely.
    // a new line fixed the problem but this is not nice.
    let dml = indoc! {r#"
        model User {
          id    Int @id
          foo   Bar Bla
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating: This line is not a valid field or attribute definition.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id    Int @id
        [1;94m 3 | [0m  [1;91mfoo   Bar Bla[0m
        [1;94m 4 | [0m}
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expectation)
}
