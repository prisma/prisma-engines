use crate::{common::*, with_header, Provider};

const SQL_SERVER: &str = r#"datasource sqlserver {
                                         provider = "sqlserver"
                                         url = "sqlserver://asdlj"
                                     }"#;

const POSTGRES: &str = r#"datasource postgres {
                                  provider = "postgres"
                                  url = "postgres://asdlj"
                                }"#;
const MYSQL: &str = r#"datasource mysql {
                                  provider = "mysql"
                                  url = "mysql://asdlj"
                                }"#;
const SQLITE: &str = r#"datasource sqlite {
                                  provider = "sqlite"
                                  url = "file:asdlj"
                                }"#;

#[test]
fn empty_index_names_are_rejected() {
    let dml = indoc! {r#"
        model User {
          id        Int    @id
          firstName String
          lastName  String

          @@index([firstName,lastName], name: "")
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The `name` argument cannot be an empty string.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m
        [1;94m 6 | [0m  [1;91m@@index([firstName,lastName], name: "")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn empty_unique_index_names_are_rejected() {
    let dml = indoc! {r#"
        model User {
          id        Int    @id
          firstName String
          lastName  String

          @@unique([firstName,lastName], name: "")
        }
    "#};

    let error = parse_unwrap_err(dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": The `name` argument cannot be an empty string.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m
        [1;94m 6 | [0m  [1;91m@@unique([firstName,lastName], name: "")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn having_both_the_map_and_name_argument_must_be_rejected() {
    let dml = with_header(
        indoc! {r#"
        model User {
          id        Int    @id
          firstName String
          lastName  String

          @@index([firstName,lastName], name: "BOTH MAP AND NAME IS NOT OK", map: "MyIndexName")
        }
    "#},
        Provider::Postgres,
        &[],
    );

    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The `@@index` attribute accepts the `name` argument as an alias for the `map` argument for legacy reasons. It does not accept both though. Please use the `map` argument to specify the database name of the index.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m
        [1;94m16 | [0m  [1;91m@@index([firstName,lastName], name: "BOTH MAP AND NAME IS NOT OK", map: "MyIndexName")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

// Namespaces SqlServer
#[test]
fn index_and_primary_cannot_have_same_name_on_sqlserver() {
    let dml = formatdoc! {r#"
        {datasource}

        model User {{
          id         Int @id(map: "MyName")
          test       Int

          @@index([test], map: "MyName")
        }}
    "#, datasource = SQL_SERVER};

    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@id": The given constraint name `MyName` has to be unique in the following namespace: on model `User` for primary key, indexes and unique constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0mmodel User {
        [1;94m 7 | [0m  id         Int @id([1;91mmap: "MyName"[0m)
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@@index": The given constraint name `MyName` has to be unique in the following namespace: on model `User` for primary key, indexes and unique constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m
        [1;94m10 | [0m  @@index([test], [1;91mmap: "MyName"[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn index_and_unique_cannot_have_same_name_on_sqlserver() {
    let dml = formatdoc! {r#"
        {datasource}

        model User {{
          id         Int
          test       Int

          @@index([test], map: "MyName")
          @@unique([id], map: "MyName")
        }}
    "#, datasource = SQL_SERVER};

    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given constraint name `MyName` has to be unique in the following namespace: on model `User` for primary key, indexes and unique constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m
        [1;94m10 | [0m  @@index([test], [1;91mmap: "MyName"[0m)
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@@unique": The given constraint name `MyName` has to be unique in the following namespace: on model `User` for primary key, indexes and unique constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m  @@index([test], map: "MyName")
        [1;94m11 | [0m  @@unique([id], [1;91mmap: "MyName"[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

// Namespaces Postgres
#[test]
fn multiple_indexes_with_same_name_are_not_supported_by_postgres() {
    let dml = formatdoc! {r#"
        {datasource}

        model User {{
          id         Int @id

          @@index([id], name: "MyIndexName")
        }}

        model Post {{
          id Int @id

          @@index([id], name: "MyIndexName")
        }}
    "#, datasource = POSTGRES};

    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given constraint name `MyIndexName` has to be unique in the following namespace: global for primary key, indexes and unique constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m
        [1;94m 9 | [0m  @@index([id], [1;91mname: "MyIndexName"[0m)
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@@index": The given constraint name `MyIndexName` has to be unique in the following namespace: global for primary key, indexes and unique constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m  @@index([id], [1;91mname: "MyIndexName"[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn unique_indexes_with_same_name_are_not_supported_by_postgres() {
    let dml = formatdoc! {r#"
        {datasource}

        model User {{
          id         Int @id
          neighborId Int

          @@index([id], map: "MyIndexName")
        }}

        model Post {{
          id Int @id
          optionId Int

          @@unique([id], map: "MyIndexName")
        }}
    "#, datasource = POSTGRES};

    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given constraint name `MyIndexName` has to be unique in the following namespace: global for primary key, indexes and unique constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m
        [1;94m10 | [0m  @@index([id], [1;91mmap: "MyIndexName"[0m)
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@@unique": The given constraint name `MyIndexName` has to be unique in the following namespace: global for primary key, indexes and unique constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m
        [1;94m17 | [0m  @@unique([id], [1;91mmap: "MyIndexName"[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn foreign_keys_and_primary_keys_with_same_name_on_same_table_are_not_supported_on_postgres() {
    let dml = formatdoc! {r#"
        {datasource}

        model A {{
          id  Int @id(map: "foo")
          bId Int
          b   B   @relation(fields: [bId], references: [id], map: "foo")
        }}
        
        model B {{
          id Int @id
          as A[]
        }}
    "#, datasource = POSTGRES};

    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@id": The given constraint name `foo` has to be unique in the following namespace: on model `A` for primary key, indexes, unique constraints and foreign keys. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0mmodel A {
        [1;94m 7 | [0m  id  Int @id([1;91mmap: "foo"[0m)
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": The given constraint name `foo` has to be unique in the following namespace: on model `A` for primary key, indexes, unique constraints and foreign keys. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  bId Int
        [1;94m 9 | [0m  b   B   @relation(fields: [bId], references: [id], [1;91mmap: "foo"[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn truncated_constraint_names_are_checked_for_uniqueness_on_postgres() {
    let dml = formatdoc! {r#"
        {datasource}

        model Post {{
          id        Int     @id @default(autoincrement())
          link LinkingTableForUserAndPostWithObnoxiouslyLongNameButNotTooLongBUTLONGER[]
        }}

        model User {{
          id    Int     @id @default(autoincrement())
          link LinkingTableForUserAndPostWithObnoxiouslyLongNameButNotTooLongBUTLONGER[]
        }}


        model LinkingTableForUserAndPostWithObnoxiouslyLongNameButNotTooLongBUTLONGER {{
          id Int     @id @default(autoincrement())

          post   Post @relation(fields: [postId], references: [id])
          postId Int          @map("post_id")

          user   User @relation(fields: [userId], references: [id])
          userId Int       @map("user_id")
        }}
    "#, datasource = POSTGRES};

    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": The given constraint name `LinkingTableForUserAndPostWithObnoxiouslyLongNameButNotToo_fkey` has to be unique in the following namespace: on model `LinkingTableForUserAndPostWithObnoxiouslyLongNameButNotTooLongBUTLONGER` for primary key, indexes, unique constraints and foreign keys. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:20[0m
        [1;94m   | [0m
        [1;94m19 | [0m
        [1;94m20 | [0m  [1;91mpost   Post @relation(fields: [postId], references: [id])[0m
        [1;94m21 | [0m  postId Int          @map("post_id")
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": The given constraint name `LinkingTableForUserAndPostWithObnoxiouslyLongNameButNotToo_fkey` has to be unique in the following namespace: on model `LinkingTableForUserAndPostWithObnoxiouslyLongNameButNotTooLongBUTLONGER` for primary key, indexes, unique constraints and foreign keys. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:23[0m
        [1;94m   | [0m
        [1;94m22 | [0m
        [1;94m23 | [0m  [1;91muser   User @relation(fields: [userId], references: [id])[0m
        [1;94m24 | [0m  userId Int       @map("user_id")
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

// Namespaces MySql
#[test]
fn foreign_keys_and_indexes_have_to_be_globally_unique_within_their_namespaces_on_mysql() {
    let dml = formatdoc! {r#"
        {datasource}

        model A {{
          id    Int @id
          bId   Int
          b     B   @relation("One", fields: [bId], references: [id], map: "test")
          bs    B[] @relation("Two")
          
          @@index([bId], map: "foo")
        }}
        
        model B {{
          id    Int @id
          aId   Int 
          a     A   @relation("Two", fields: [aId], references: [id], map: "test")          
          as    A[] @relation("One")

          @@index([aId], map: "foo")
        }}
    "#, datasource = MYSQL};

    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": The given constraint name `test` has to be unique in the following namespace: global for foreign keys. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  bId   Int
        [1;94m 9 | [0m  b     B   @relation("One", fields: [bId], references: [id], [1;91mmap: "test"[0m)
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": The given constraint name `test` has to be unique in the following namespace: global for foreign keys. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:18[0m
        [1;94m   | [0m
        [1;94m17 | [0m  aId   Int 
        [1;94m18 | [0m  a     A   @relation("Two", fields: [aId], references: [id], [1;91mmap: "test"[0m)          
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

// Namespaces Sqlite

#[test]
fn multiple_indexes_with_same_name_are_not_supported_by_sqlite() {
    let dml = formatdoc! {r#"
        {datasource}
        
        model User {{
          id         Int @id
          neighborId Int

          @@index([id], name: "MyIndexName")
        }}

        model Post {{
          id Int @id
          optionId Int

          @@index([id], name: "MyIndexName")
        }}
    "#, datasource = SQLITE};

    let error = parse_unwrap_err(&dml);

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@index": The given constraint name `MyIndexName` has to be unique in the following namespace: global for indexes and unique constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m
        [1;94m10 | [0m  @@index([id], [1;91mname: "MyIndexName"[0m)
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@@index": The given constraint name `MyIndexName` has to be unique in the following namespace: global for indexes and unique constraints. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m
        [1;94m17 | [0m  @@index([id], [1;91mname: "MyIndexName"[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}
