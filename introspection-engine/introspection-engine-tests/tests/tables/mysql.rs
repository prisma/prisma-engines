use barrel::types;
use indoc::indoc;
use introspection_engine_tests::test_api::*;

#[test_connector(tags(Mysql))]
async fn a_table_with_non_id_autoincrement(api: &TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE `Test` (
            `id` INTEGER PRIMARY KEY,
            `authorId` INTEGER AUTO_INCREMENT UNIQUE
        );
    "#;

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model Test {
          id       Int @id
          authorId Int @unique(map: "authorId") @default(autoincrement())
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
//TODO(matthias) the default("") is weird, check where this comes from
async fn a_table_with_partial_primary_key(api: &TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE `Test` (
            `id` varchar(3000),
            Primary Key(`id`(100))
        );
        
        CREATE TABLE `Test2` (
            `id_1` varchar(3000),
            `id_2` varchar(3000),
            Primary Key(`id_1`(100), `id_2`(10))
        );
    "#;

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model Test {
          id String @id(length: 100) @db.VarChar(3000)
        }
        
        model Test2 {
          id_1 String @db.VarChar(3000)
          id_2 String @db.VarChar(3000)
        
          @@id([id_1(length: 100), id_2(length: 10)])
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn partial_indexes_should_work_on_mysql(api: &TestApi) -> TestResult {
    api.barrel()
        .execute(|migration| {
            migration.create_table("Blog", move |t| {
                t.add_column("id", types::primary());
                t.add_column("int_col", types::integer());
                t.inject_custom("blob_col mediumblob");
                t.inject_custom("Index `partial_blob_col_index` (blob_col(10))");
                t.inject_custom("Index `partial_compound` (blob_col(11), int_col)");
            });
        })
        .await?;

    let dm = indoc! {r##"
        model Blog {
          id                Int     @id @default(autoincrement())
          int_col           Int
          blob_col          Bytes?  @db.MediumBlob

          @@index([blob_col(length: 10)], map: "partial_blob_col_index")
          @@index([blob_col(length: 11), int_col], map: "partial_compound")
        }
    "##};

    let result = &api.introspect().await?;
    api.assert_eq_datamodels(dm, result);

    Ok(())
}

#[test_connector(tags(Mysql))]
async fn a_table_with_partial_and_sorted_indices(api: &TestApi) -> TestResult {
    let setup = r#"
        CREATE TABLE `Test` (
            `id` varchar(3000) Not Null,
            Unique(`id`(100))
        );
        
        CREATE TABLE `Test2` (
            `id_1` varchar(3000) Not Null,
            `id_2` varchar(3000) Not Null,
            Unique(`id_1`(100) DESC, `id_2`(10) ASC)
        );
    "#;

    api.raw_cmd(setup).await;

    let expected = expect![[r#"
        model Test {
          id String @unique(map: "id", length: 100) @db.VarChar(3000)
        }
        
        model Test2 {
          id_1 String @db.VarChar(3000)
          id_2 String @db.VarChar(3000)
        
          @@unique([id_1(length: 100, sort: Desc), id_2(length: 10)], map: "id_1")
        }
    "#]];

    expected.assert_eq(&api.introspect_dml().await?);

    Ok(())
}
