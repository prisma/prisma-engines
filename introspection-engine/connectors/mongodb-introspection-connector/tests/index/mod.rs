use crate::common::*;
use bson::{doc, Bson};
use expect_test::expect;
use mongodb::{options::IndexOptions, IndexModel};

#[test]
fn single_column_normal_index() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti", "age": 9}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "age": 1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id   String @id @default(dbgenerated()) @map("_id") @db.ObjectId
          age  Int
          name String

          @@index([age], map: "age_1")
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn single_column_normal_index_default_name() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti", "age": 9}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder()
            .unique(Some(false))
            .name(Some("A_age_idx".to_string()))
            .build();

        let model = IndexModel::builder()
            .keys(doc! { "age": 1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id   String @id @default(dbgenerated()) @map("_id") @db.ObjectId
          age  Int
          name String

          @@index([age])
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn multi_column_normal_index() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti", "age": 9}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "age": 1, "name": -1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id   String @id @default(dbgenerated()) @map("_id") @db.ObjectId
          age  Int
          name String

          @@index([age, name], map: "age_1_name_-1")
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn single_column_unique_index() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti", "age": 9}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(true)).build();

        let model = IndexModel::builder()
            .keys(doc! { "age": 1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id   String @id @default(dbgenerated()) @map("_id") @db.ObjectId
          age  Int    @unique(map: "age_1")
          name String
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn single_column_unique_index_default_name() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti", "age": 9}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder()
            .unique(Some(true))
            .name(Some("A_age_key".to_string()))
            .build();

        let model = IndexModel::builder()
            .keys(doc! { "age": 1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id   String @id @default(dbgenerated()) @map("_id") @db.ObjectId
          age  Int    @unique
          name String
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn multi_column_unique_index() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti", "age": 9}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(true)).build();

        let model = IndexModel::builder()
            .keys(doc! { "age": 1, "name": -1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id   String @id @default(dbgenerated()) @map("_id") @db.ObjectId
          age  Int
          name String

          @@unique([age, name], map: "age_1_name_-1")
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn unsupported_types_in_a_unique_index() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"data": Bson::JavaScriptCode("let a = 1 + 1;".to_string())}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(true)).build();

        let model = IndexModel::builder()
            .keys(doc! { "data": 1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id   String                        @id @default(dbgenerated()) @map("_id") @db.ObjectId
          data Unsupported("JavaScriptCode") @unique(map: "data_1")
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn unsupported_types_in_an_index() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"data": Bson::JavaScriptCode("let a = 1 + 1;".to_string())}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "data": 1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id   String                        @id @default(dbgenerated()) @map("_id") @db.ObjectId
          data Unsupported("JavaScriptCode")

          @@index([data], map: "data_1")
        }
    "#]];

    expected.assert_eq(res.datamodel());

    res.assert_warning(
        "These fields are not supported by the Prisma Client, because Prisma currently does not support their types.",
    );
}
