use crate::{common::*, with_header};

#[test]
fn composite_types_are_parsed_without_error() {
    let datamodel = r#"
        datasource db{
            provider = "mongodb"
            url = "mongo+srv:/...."
        }

        type Address {
            name String?
            street String @db.ObjectId
        }

        model User {
            id  String @id @default(auto()) @map("_id") @db.ObjectId
            address Address?
        }
    "#;

    assert_valid(datamodel);
}

#[test]
fn composite_types_must_have_at_least_one_visible_field() {
    let schema = indoc! {r#"
        datasource mongodb {
            provider = "mongodb"
            url = env("TEST_DATABASE_URL")
        }

        type Address {
          // name String?
        }
    "#};

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: A type must have at least one field defined.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m
        [1;94m 6 | [0m[1;91mtype Address {[0m
        [1;94m 7 | [0m  // name String?
        [1;94m 8 | [0m}
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expected);
}

#[test]
fn composite_types_can_nest() {
    let schema = r#"
        datasource db {
            provider = "mongodb"
            url = "mongodb://"
        }

        type Address {
            name String?
            secondaryAddress Address?
        }
    "#;

    assert_valid(schema);
}

#[test]
fn required_cycles_to_self_are_not_allowed() {
    let datamodel = indoc! {r#"
        datasource db {
          provider = "mongodb"
          url = "mongodb://"
        }

        type Address {
          name String?
          secondaryAddress Address
        }
    "#};

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating field `secondaryAddress` in composite type `Address`: The type is the same as the parent and causes an endless cycle. Please change the field to be either optional or a list.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  name String?
        [1;94m 8 | [0m  [1;91msecondaryAddress Address[0m
        [1;94m 9 | [0m}
        [1;94m   | [0m
    "#]];

    expect_error(datamodel, &expected);
}

#[test]
fn list_cycles_to_self_are_allowed() {
    let datamodel = indoc! {r#"
        datasource db {
          provider = "mongodb"
          url = "mongodb://"
        }

        type Address {
          name String?
          secondaryAddresses Address[]
        }
    "#};

    assert_valid(datamodel);
}

#[test]
fn required_cycles_are_not_allowed() {
    let datamodel = indoc! {r#"
        datasource db {
          provider = "mongodb"
          url = "mongodb://"
        }

        type PostCode {
          code Int
        }

        type Address {
          name String?
          city City
          code PostCode
        }

        type City {
          name         String?
          worldAddress Address
        }
    "#};

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating field `worldAddress` in composite type `City`: The types cause an endless cycle in the path `City` → `Address` → `City`. Please change one of the fields to be either optional or a list to break the cycle.[0m
          [1;94m-->[0m  [4mschema.prisma:18[0m
        [1;94m   | [0m
        [1;94m17 | [0m  name         String?
        [1;94m18 | [0m  [1;91mworldAddress Address[0m
        [1;94m19 | [0m}
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating field `city` in composite type `Address`: The types cause an endless cycle in the path `Address` → `City` → `Address`. Please change one of the fields to be either optional or a list to break the cycle.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0m  name String?
        [1;94m12 | [0m  [1;91mcity City[0m
        [1;94m13 | [0m  code PostCode
        [1;94m   | [0m
    "#]];

    expect_error(datamodel, &expected);
}

#[test]
fn cycles_broken_with_an_optional_are_allowed() {
    let datamodel = indoc! {r#"
        datasource db {
          provider = "mongodb"
          url = "mongodb://"
        }

        type PostCode {
          code Int
        }

        type Address {
          name String?
          city City
          code PostCode
        }

        type City {
          name         String?
          worldAddress Address?
        }
    "#};

    assert_valid(datamodel);
}

#[test]
fn unsupported_should_work() {
    let schema = indoc! {r#"
        datasource mongodb {
            provider = "mongodb"
            url = env("TEST_DATABASE_URL")
        }

        type A {
          field Unsupported("Unknown")
        }
    "#};

    assert_valid(schema);
}

#[test]
fn block_level_map_not_allowed() {
    let schema = indoc! {r#"
        type A {
          field Int

          @@map("foo")
        }

        model B {
          id Int @id
          a  A
        }
    "#};

    let dml = with_header(schema, crate::Provider::Mongo, &[]);
    let error = parse_unwrap_err(&dml);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: The name of a composite type is not persisted in the database, therefore it does not need a mapped database name.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m
        [1;94m14 | [0m  [1;91m@@map("foo")[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}
