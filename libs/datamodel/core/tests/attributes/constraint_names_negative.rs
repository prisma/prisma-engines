use crate::attributes::with_postgres_provider;
use crate::common::*;

const SQL_SERVER: &'static str = r#"datasource sqlserver {
                                         provider = "sqlserver"
                                         url = "sqlserver://asdlj"
                                     }"#;

const POSTGRES: &'static str = r#"datasource postgres {
                                  provider = "postgres"
                                  url = "postgres://asdlj"
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

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The `name` argument cannot be an empty string.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m
        [1;94m 6 | [0m  @@[1;91mindex([firstName,lastName], name: "")[0m
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

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": The `name` argument cannot be an empty string.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m
        [1;94m 6 | [0m  @@[1;91munique([firstName,lastName], name: "")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn having_both_the_map_and_name_argument_must_be_rejected() {
    let dml = with_postgres_provider(indoc! {r#"
        model User {
          id        Int    @id
          firstName String
          lastName  String

          @@index([firstName,lastName], name: "BOTH MAP AND NAME IS NOT OK", map: "MyIndexName")
        }
    "#});

    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The `@@index` attribute accepts the `name` argument as an alias for the `map` argument for legacy reasons. It does not accept both though. Please use the `map` argument to specify the database name of the index.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m
        [1;94m16 | [0m  @@[1;91mindex([firstName,lastName], name: "BOTH MAP AND NAME IS NOT OK", map: "MyIndexName")[0m
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

    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@id": The given constraint name `MyName` has to be unique in the following namespace: model User. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0mmodel User {
        [1;94m 7 | [0m  id         Int @[1;91mid(map: "MyName")[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@index": The given constraint name `MyName` has to be unique in the following namespace: model User. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m
        [1;94m 6 | [0m[1;91mmodel User {[0m
        [1;94m 7 | [0m  id         Int @id(map: "MyName")
        [1;94m 8 | [0m  test       Int
        [1;94m 9 | [0m
        [1;94m10 | [0m  @@index([test], map: "MyName")
        [1;94m11 | [0m}
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

    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The given constraint name `MyName` has to be unique in the following namespace: model User. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m
        [1;94m 6 | [0m[1;91mmodel User {[0m
        [1;94m 7 | [0m  id         Int
        [1;94m 8 | [0m  test       Int
        [1;94m 9 | [0m
        [1;94m10 | [0m  @@index([test], map: "MyName")
        [1;94m11 | [0m  @@unique([id], map: "MyName")
        [1;94m12 | [0m}
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@unique": The given constraint name `MyName` has to be unique in the following namespace: model User. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m
        [1;94m 6 | [0m[1;91mmodel User {[0m
        [1;94m 7 | [0m  id         Int
        [1;94m 8 | [0m  test       Int
        [1;94m 9 | [0m
        [1;94m10 | [0m  @@index([test], map: "MyName")
        [1;94m11 | [0m  @@unique([id], map: "MyName")
        [1;94m12 | [0m}
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

    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@index": The given constraint name `MyIndexName` has to be unique in the following namespace: global. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m
        [1;94m 6 | [0m[1;91mmodel User {[0m
        [1;94m 7 | [0m  id         Int @id
        [1;94m 8 | [0m
        [1;94m 9 | [0m  @@index([id], name: "MyIndexName")
        [1;94m10 | [0m}
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@index": The given constraint name `MyIndexName` has to be unique in the following namespace: global. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0m
        [1;94m12 | [0m[1;91mmodel Post {[0m
        [1;94m13 | [0m  id Int @id
        [1;94m14 | [0m
        [1;94m15 | [0m  @@index([id], name: "MyIndexName")
        [1;94m16 | [0m}
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

    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe index name `MyIndexName` is declared multiple times. With the current connector index names have to be globally unique.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m
        [1;94m17 | [0m  @@[1;91munique([id], map: "MyIndexName")[0m
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

    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@id": The given constraint name `foo` has to be unique in the following namespace: pk, key, idx, fk on A. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0mmodel A {
        [1;94m 7 | [0m  id  Int @[1;91mid(map: "foo")[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": The given constraint name `foo` has to be unique in the following namespace: pk, key, idx, fk on A. Please provide a different name using the `map` argument.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  bId Int
        [1;94m 9 | [0m  b   B   @relation(fields: [bId], references: [id], [1;91mmap: "foo"[0m)
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

// Namespaces MySql

// Namespaces Sqlite

#[test]
fn multiple_indexes_with_same_name_are_not_supported_by_sqlite() {
    let dml = indoc! {r#"
        datasource sqlite {
          provider = "sqlite"
          url = "sqlite://asdlj"
        }

        model User {
          id         Int @id
          neighborId Int

          @@index([id], name: "MyIndexName")
        }

        model Post {
          id Int @id
          optionId Int

          @@index([id], name: "MyIndexName")
        }
    "#};

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe index name `MyIndexName` is declared multiple times. With the current connector index names have to be globally unique.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m
        [1;94m17 | [0m  @@[1;91mindex([id], name: "MyIndexName")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}
