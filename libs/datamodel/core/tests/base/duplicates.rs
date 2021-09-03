use crate::common::*;

#[test]
fn fail_on_duplicate_models() {
    let dml = indoc! {r#"
        model User {
          id Int @id
        }

        model User {
          id Int @id
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe model "User" cannot be defined because a model with that name already exists.[0m
          [1;94m-->[0m  [4mschema.prisma:5[0m
        [1;94m   | [0m
        [1;94m 4 | [0m
        [1;94m 5 | [0mmodel [1;91mUser[0m {
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&parse_and_render_error(dml));
}

#[test]
fn fail_on_duplicate_models_with_map() {
    let dml = indoc! {r#"
        model Customer {
          id Int @id

          @@map("User")
        }

        model User {
          id Int @id
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe model with database name "User" could not be defined because another model with this name exists: "User"[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m
        [1;94m 4 | [0m  @@[1;91mmap("User")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&parse_and_render_error(dml));
}

// From issue: https://github.com/prisma/prisma/issues/1988
#[test]
fn fail_on_duplicate_models_with_relations() {
    let dml = indoc! {r#"
        model Post {
          id Int @id
        }

        model Post {
          id Int @id
          categories Categories[]
        }

        model Categories {
          post Post @relation(fields:[postId], references: [id])
          postId Int
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe model "Post" cannot be defined because a model with that name already exists.[0m
          [1;94m-->[0m  [4mschema.prisma:5[0m
        [1;94m   | [0m
        [1;94m 4 | [0m
        [1;94m 5 | [0mmodel [1;91mPost[0m {
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&parse_and_render_error(dml));
}

#[test]
fn fail_on_model_enum_conflict() {
    let dml = indoc! {r#"
        enum User {
          Admin
          Moderator
        }
        model User {
          id Int @id
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe model "User" cannot be defined because a enum with that name already exists.[0m
          [1;94m-->[0m  [4mschema.prisma:5[0m
        [1;94m   | [0m
        [1;94m 4 | [0m}
        [1;94m 5 | [0mmodel [1;91mUser[0m {
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&parse_and_render_error(dml));
}
#[test]
fn fail_on_model_type_conflict() {
    let dml = indoc! {r#"
        type User = String

        model User {
          id Int @id
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe model "User" cannot be defined because a type with that name already exists.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m
        [1;94m 3 | [0mmodel [1;91mUser[0m {
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&parse_and_render_error(dml));
}

#[test]
fn fail_on_enum_type_conflict() {
    let dml = indoc! {r#"
        type User = String

        enum User {
          Admin
          Moderator
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe enum "User" cannot be defined because a type with that name already exists.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m
        [1;94m 3 | [0menum [1;91mUser[0m {
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&parse_and_render_error(dml));
}

#[test]
fn fail_on_duplicate_field() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          firstName String
          firstName String
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mField "firstName" is already defined on model "User".[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m  firstName String
        [1;94m 4 | [0m  [1;91mfirstName[0m String
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&parse_and_render_error(dml));
}

#[test]
fn fail_on_duplicate_field_with_map() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          firstName String
          otherName String @map("firstName")
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mField "otherName" is already defined on model "User".[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m  firstName String
        [1;94m 4 | [0m  [1;91motherName String @map("firstName")[0m
        [1;94m 5 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&parse_and_render_error(dml));
}

#[test]
fn fail_on_duplicate_mapped_field_name() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          firstName String @map("thename")
          lastName String @map("thename")
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mField "lastName" is already defined on model "User".[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m  firstName String @map("thename")
        [1;94m 4 | [0m  [1;91mlastName String @map("thename")[0m
        [1;94m 5 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&parse_and_render_error(dml));
}

#[test]
fn fail_on_duplicate_enum_value() {
    let dml = indoc! {r#"
        enum Role {
          Admin
          Moderator
          Moderator
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mValue "Moderator" is already defined on enum "Role".[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m  Moderator
        [1;94m 4 | [0m  [1;91mModerator[0m
        [1;94m 5 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&parse_and_render_error(dml));
}

#[test]
fn fail_on_reserved_name_for_enum() {
    let dml = indoc! {r#"
        enum String {
          Admin
          Moderator
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1m"String" is a reserved scalar type name and cannot be used.[0m
          [1;94m-->[0m  [4mschema.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0menum [1;91mString[0m {
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&parse_and_render_error(dml));
}

#[test]
fn fail_on_reserved_name_for_model() {
    let dml = indoc! {r#"
        model DateTime {
          id Int @id
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1m"DateTime" is a reserved scalar type name and cannot be used.[0m
          [1;94m-->[0m  [4mschema.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0mmodel [1;91mDateTime[0m {
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&parse_and_render_error(dml));
}

#[test]
fn fail_on_reserved_name_for_custom_type() {
    let dml = "type Int = String";

    let expectation = expect![[r#"
        [1;91merror[0m: [1m"Int" is a reserved scalar type name and cannot be used.[0m
          [1;94m-->[0m  [4mschema.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0mtype [1;91mInt[0m = String
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&parse_and_render_error(dml));
}

#[test]
fn multiple_indexes_with_same_autogenerated_name_trigger_datamodel_validation() {
    let dml = indoc! {r#"
        model User {
          id     Int    @id @default(autoincrement())
          email  String
          name   String

          @@unique([email, name])
          @@unique([email, name])
        }
     "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe index name `User_email_name_key` is declared multiple times. With the current connector index names have to be globally unique.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m  @@unique([email, name])
        [1;94m 7 | [0m  @@[1;91munique([email, name])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&parse_and_render_error(dml));
}

#[test]
fn multiple_indexes_with_same_autogenerated_name_trigger_datamodel_validation_new() {
    let dml = indoc! {r#"
        datasource test {
          provider = "postgres"
          url = "postgresql://..."
        }
           
        generator js {
           provider = "prisma-client-js"
           previewFeatures = ["namedConstraints"]
        }   
    
        model User {
          id     Int    @id @default(autoincrement())
          email  String
          name   String

          @@unique([email, name])
          @@unique([email, name])
        }
     "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe index name `User_email_name_key` is declared multiple times. With the current connector index names have to be globally unique.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  @@unique([email, name])
        [1;94m17 | [0m  @@[1;91munique([email, name])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&parse_and_render_error(dml));
}
