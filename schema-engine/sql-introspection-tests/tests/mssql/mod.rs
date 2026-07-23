use expect_test::expect;
use quaint::prelude::Queryable;
use sql_introspection_tests::{TestResult, test_api::*};
use test_macros::test_connector;

#[test_connector(tags(Mssql))]
async fn user_defined_type_aliases_should_map_to_the_system_type(api: &mut TestApi) -> TestResult {
    let create_type = format!("CREATE TYPE [{}].[Name] FROM [nvarchar](50) NULL", api.schema_name());
    api.database().raw_cmd(&create_type).await?;

    let create_table = format!(
        r#"
        CREATE TABLE [{schema_name}].[A] (
            id INT IDENTITY,
            name [{schema_name}].[Name],
            CONSTRAINT [A_pkey] PRIMARY KEY (id),
        )"#,
        schema_name = api.schema_name()
    );

    api.database().raw_cmd(&create_table).await?;

    let result = api.introspect_dml().await?;

    let expected = expect![[r#"
        model A {
          id   Int     @id @default(autoincrement())
          name String? @db.NVarChar(50)
        }
    "#]];

    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn ms_xml_indexes_are_skipped(api: &mut TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE [$schema].[xml_test] (
            id INT IDENTITY,
            data XML,

            CONSTRAINT [xml_test_pkey] PRIMARY KEY (id)
        );

        CREATE PRIMARY XML INDEX primaryIndex ON [$schema].[xml_test] (data);
        CREATE XML INDEX secondaryIndex ON [$schema].[xml_test] (data) USING XML INDEX primaryIndex FOR PATH;
    "#
    .replace("$schema", api.schema_name());

    api.raw_cmd(&setup).await;

    let expected = expect![[r#"
        model xml_test {
          id   Int     @id @default(autoincrement())
          data String? @db.Xml
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn non_standard_id_clustering(api: &mut TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE [$schema].[test] (
            id INT IDENTITY,
            CONSTRAINT [test_pkey] PRIMARY KEY NONCLUSTERED (id)
        );
    "#
    .replace("$schema", api.schema_name());

    api.raw_cmd(&setup).await;

    let expected = expect![[r#"
        model test {
          id Int @id(clustered: false) @default(autoincrement())
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn standard_id_clustering(api: &mut TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE [$schema].[test] (
            id INT IDENTITY,
            CONSTRAINT [test_pkey] PRIMARY KEY CLUSTERED (id)
        );
    "#
    .replace("$schema", api.schema_name());

    api.raw_cmd(&setup).await;

    let expected = expect![[r#"
        model test {
          id Int @id @default(autoincrement())
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn non_standard_compound_id_clustering(api: &mut TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE [$schema].[test] (
            a INT,
            b INT,
            CONSTRAINT [test_pkey] PRIMARY KEY NONCLUSTERED (a, b)
        );
    "#
    .replace("$schema", api.schema_name());

    api.raw_cmd(&setup).await;

    let expected = expect![[r#"
        model test {
          a Int
          b Int

          @@id([a, b], clustered: false)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn standard_compound_id_clustering(api: &mut TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE [$schema].[test] (
            a INT,
            b INT,
            CONSTRAINT [test_pkey] PRIMARY KEY CLUSTERED (a, b)
        );
    "#
    .replace("$schema", api.schema_name());

    api.raw_cmd(&setup).await;

    let expected = expect![[r#"
        model test {
          a Int
          b Int

          @@id([a, b])
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn non_standard_unique_clustering(api: &mut TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE [$schema].[test] (
            a INT NOT NULL,
            CONSTRAINT [test_a_key] UNIQUE CLUSTERED (a)
        );
    "#
    .replace("$schema", api.schema_name());

    api.raw_cmd(&setup).await;

    let expected = expect![[r#"
        model test {
          a Int @unique(clustered: true)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn standard_unique_clustering(api: &mut TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE [$schema].[test] (
            a INT NOT NULL,
            CONSTRAINT [test_a_key] UNIQUE NONCLUSTERED (a)
        );
    "#
    .replace("$schema", api.schema_name());

    api.raw_cmd(&setup).await;

    let expected = expect![[r#"
        model test {
          a Int @unique
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn non_standard_compound_unique_clustering(api: &mut TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE [$schema].[test] (
            a INT NOT NULL,
            b INT NOT NULL,
            CONSTRAINT [test_a_b_key] UNIQUE CLUSTERED (a, b)
        );
    "#
    .replace("$schema", api.schema_name());

    api.raw_cmd(&setup).await;

    let expected = expect![[r#"
        model test {
          a Int
          b Int

          @@unique([a, b], clustered: true)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn standard_compound_unique_clustering(api: &mut TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE [$schema].[test] (
            a INT NOT NULL,
            b INT NOT NULL,
            CONSTRAINT [test_a_b_key] UNIQUE NONCLUSTERED (a, b)
        );
    "#
    .replace("$schema", api.schema_name());

    api.raw_cmd(&setup).await;

    let expected = expect![[r#"
        model test {
          a Int
          b Int

          @@unique([a, b])
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn non_standard_index_clustering(api: &mut TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE [$schema].[test] (
            id INT IDENTITY,
            a INT NOT NULL,
            CONSTRAINT [test_pkey] PRIMARY KEY NONCLUSTERED (id)
        );

        CREATE CLUSTERED INDEX [test_a_idx] ON [$schema].[test] (a);
    "#
    .replace("$schema", api.schema_name());

    api.raw_cmd(&setup).await;

    let expected = expect![[r#"
        model test {
          id Int @id(clustered: false) @default(autoincrement())
          a  Int

          @@index([a], clustered: true)
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn standard_index_clustering(api: &mut TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE [$schema].[test] (
            id INT IDENTITY,
            a INT NOT NULL,
            CONSTRAINT [test_pkey] PRIMARY KEY CLUSTERED (id)
        );

        CREATE NONCLUSTERED INDEX [test_a_idx] ON [$schema].[test] (a);
    "#
    .replace("$schema", api.schema_name());

    api.raw_cmd(&setup).await;

    let expected = expect![[r#"
        model test {
          id Int @id @default(autoincrement())
          a  Int

          @@index([a])
        }
    "#]];

    let result = api.introspect_dml().await?;
    expected.assert_eq(&result);

    Ok(())
}
