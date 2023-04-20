use crate::{common::*, with_header, Provider};

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
        [1;91merror[0m: [1mThe model with database name "User" could not be defined because another model or view with this name exists: "User"[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m
        [1;94m 4 | [0m  [1;91m@@map("User")[0m
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
fn fail_on_duplicate_composite_types() {
    let dml = indoc! {r#"
        datasource db {
            provider = "mongodb"
            url = "mongodb://"
        }

        type Address {
            street String
        }

        type Address {
            name String
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe composite type "Address" cannot be defined because a composite type with that name already exists.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m
        [1;94m10 | [0mtype [1;91mAddress[0m {
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&parse_and_render_error(dml));
}

#[test]
fn fail_on_composite_type_model_conflict() {
    let dml = indoc! {r#"
        datasource db {
            provider = "mongodb"
            url = "mongodb://"
        }

        type Address {
            street String
        }

        model Address {
            id Int @id
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe model "Address" cannot be defined because a composite type with that name already exists.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m
        [1;94m10 | [0mmodel [1;91mAddress[0m {
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&parse_and_render_error(dml));
}

#[test]
fn fail_on_composite_type_enum_conflict() {
    let dml = indoc! {r#"
        datasource db {
            provider = "mongodb"
            url = "mongodb://"
        }

        type Address {
            street String
        }

        enum Address {
            HERE
            THERE
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe enum "Address" cannot be defined because a composite type with that name already exists.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m
        [1;94m10 | [0menum [1;91mAddress[0m {
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
fn fail_on_duplicate_model_field() {
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
fn fail_on_duplicate_composite_type_field() {
    let dml = indoc! {r#"
        datasource db {
            provider = "mongodb"
            url = "mongodb://"
        }

        type Address {
          name String
          street String
          street String
          number Int
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mField "street" is already defined on composite type "Address".[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  street String
        [1;94m 9 | [0m  [1;91mstreet[0m String
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
fn do_not_fail_on_duplicate_field_with_map_if_the_maps_differ() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          firstName String @map("thirdName")
          otherName String @map("firstName")
        }
    "#};

    assert_valid(dml);
}

#[test]
fn fail_on_duplicate_composite_type_field_with_map() {
    let dml = indoc! {r#"
        datasource db {
            provider = "mongodb"
            url = "mongodb://"
        }

        type User {
            firstName String
            otherName String @map("firstName")
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mField "otherName" is already defined on composite type "User".[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m    firstName String
        [1;94m 8 | [0m    [1;91motherName String @map("firstName")[0m
        [1;94m 9 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&parse_and_render_error(dml));
}

#[test]
fn mapped_names_should_not_cause_collisions_with_names() {
    let schema = indoc! {r#"
        type TestData {
          id  String @map("_id")
          id_ String @map("id")
        }
    "#};

    let schema = psl::parse_schema(with_header(schema, Provider::Mongo, &[])).unwrap();
    let typ = schema.assert_has_type("TestData");

    typ.assert_has_scalar_field("id");
    typ.assert_has_scalar_field("id_");
}

#[test]
fn fail_on_duplicate_mapped_composite_type_field() {
    let dml = indoc! {r#"
        datasource db {
            provider = "mongodb"
            url = "mongodb://"
        }

        type User {
            primaryName String @map("firstName")
            otherName String @map("firstName")
        }
    "#};

    let expectation = expect![[r#"
        [1;91merror[0m: [1mField "firstName" is already defined on composite type "User".[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m    primaryName String @map("firstName")
        [1;94m 8 | [0m    [1;91motherName String @map("firstName")[0m
        [1;94m 9 | [0m}
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
fn multiple_indexes_with_same_autogenerated_name_trigger_datamodel_validation() {
    let dml = indoc! {r#"
        datasource test {
          provider = "postgres"
          url = "postgresql://..."
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
        [1;91merror[0m: [1mError parsing attribute "@@unique": The given constraint name `User_email_name_key` has to be unique in the following namespace: global for primary key, indexes and unique constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m  [1;91m@@unique([email, name])[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@@unique": The given constraint name `User_email_name_key` has to be unique in the following namespace: global for primary key, indexes and unique constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0m  @@unique([email, name])
        [1;94m12 | [0m  [1;91m@@unique([email, name])[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&parse_and_render_error(dml));
}
