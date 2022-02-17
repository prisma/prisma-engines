use crate::test_api::*;
use datamodel::common::preview_features::PreviewFeature;
use introspection_connector::CompositeTypeDepth;
use mongodb::{
    bson::{doc, Bson},
    options::IndexOptions,
    IndexModel,
};

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
          id   String @id @default(auto()) @map("_id") @db.ObjectId
          age  Int
          name String

          @@index([age], map: "age_1")
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn single_column_descending_index_no_preview_enabled() {
    let depth = CompositeTypeDepth::Infinite;
    let features = PreviewFeature::MongoDb;

    let res = introspect_features(depth, features.into(), |db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti", "age": 9}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "age": -1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id   String @id @default(auto()) @map("_id") @db.ObjectId
          age  Int
          name String

          @@index([age], map: "age_-1")
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn single_column_descending_index() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti", "age": 9}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "age": -1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id   String @id @default(auto()) @map("_id") @db.ObjectId
          age  Int
          name String

          @@index([age(sort: Desc)], map: "age_-1")
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn single_column_fulltext_index() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti", "age": 9}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "name": "text" })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id   String @id @default(auto()) @map("_id") @db.ObjectId
          age  Int
          name String

          @@fulltext([name], map: "name_\"text\"")
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn multi_column_fulltext_index() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti", "title": "cat", "age": 9}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "name": "text", "title": "text" })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id    String @id @default(auto()) @map("_id") @db.ObjectId
          age   Int
          name  String
          title String

          @@fulltext([name, title], map: "name_\"text\"_title_\"text\"")
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn multi_column_fulltext_index_with_desc_in_end() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti", "title": "cat", "age": 9}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "name": "text", "title": "text", "age": -1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id    String @id @default(auto()) @map("_id") @db.ObjectId
          age   Int
          name  String
          title String

          @@fulltext([name, title, age(sort: Desc)], map: "name_\"text\"_title_\"text\"_age_-1")
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn multi_column_fulltext_index_with_desc_in_beginning() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti", "title": "cat", "age": 9}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "age": -1, "name": "text", "title": "text" })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id    String @id @default(auto()) @map("_id") @db.ObjectId
          age   Int
          name  String
          title String

          @@fulltext([age(sort: Desc), name, title], map: "age_-1_name_\"text\"_title_\"text\"")
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn multi_column_fulltext_index_with_asc_in_end() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti", "title": "cat", "age": 9}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "name": "text", "title": "text", "age": 1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id    String @id @default(auto()) @map("_id") @db.ObjectId
          age   Int
          name  String
          title String

          @@fulltext([name, title, age(sort: Asc)], map: "name_\"text\"_title_\"text\"_age_1")
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn multi_column_fulltext_index_with_asc_in_beginning() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti", "title": "cat", "age": 9}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "age": 1, "name": "text", "title": "text" })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id    String @id @default(auto()) @map("_id") @db.ObjectId
          age   Int
          name  String
          title String

          @@fulltext([age(sort: Asc), name, title], map: "age_1_name_\"text\"_title_\"text\"")
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn multi_column_fulltext_index_with_asc_in_beginning_desc_in_end() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;

        let collection = db.collection("A");
        let docs = vec![doc! { "name": "Musti", "title": "cat", "age": 9, "weight": 5 }];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder()
            .unique(Some(false))
            .name(Some("long_name".to_string()))
            .build();

        let model = IndexModel::builder()
            .keys(doc! { "age": 1, "name": "text", "title": "text", "weight": -1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id     String @id @default(auto()) @map("_id") @db.ObjectId
          age    Int
          name   String
          title  String
          weight Int

          @@fulltext([age(sort: Asc), name, title, weight(sort: Desc)], map: "long_name")
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn fultext_index_without_preview_flag() {
    let depth = CompositeTypeDepth::Infinite;
    let features = PreviewFeature::MongoDb;

    let res = introspect_features(depth, features.into(), |db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti", "age": 9}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "name": "text" })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id   String @id @default(auto()) @map("_id") @db.ObjectId
          age  Int
          name String
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn index_pointing_to_a_renamed_field() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti", "_age": 9}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "_age": 1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id   String @id @default(auto()) @map("_id") @db.ObjectId
          age  Int    @map("_age")
          name String

          @@index([age], map: "_age_1")
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
          id   String @id @default(auto()) @map("_id") @db.ObjectId
          age  Int
          name String

          @@index([age])
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn multi_column_normal_index_no_preview() {
    let depth = CompositeTypeDepth::Infinite;
    let features = PreviewFeature::MongoDb;

    let res = introspect_features(depth, features.into(), |db| async move {
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
          id   String @id @default(auto()) @map("_id") @db.ObjectId
          age  Int
          name String

          @@index([age, name], map: "age_1_name_-1")
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
          id   String @id @default(auto()) @map("_id") @db.ObjectId
          age  Int
          name String

          @@index([age, name(sort: Desc)], map: "age_1_name_-1")
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
          id   String @id @default(auto()) @map("_id") @db.ObjectId
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
          id   String @id @default(auto()) @map("_id") @db.ObjectId
          age  Int    @unique
          name String
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn multi_column_unique_index_no_preview() {
    let depth = CompositeTypeDepth::Infinite;
    let features = PreviewFeature::MongoDb;

    let res = introspect_features(depth, features.into(), |db| async move {
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
          id   String @id @default(auto()) @map("_id") @db.ObjectId
          age  Int
          name String

          @@unique([age, name], map: "age_1_name_-1")
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
          id   String @id @default(auto()) @map("_id") @db.ObjectId
          age  Int
          name String

          @@unique([age, name(sort: Desc)], map: "age_1_name_-1")
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
          id   String                        @id @default(auto()) @map("_id") @db.ObjectId
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
          id   String                        @id @default(auto()) @map("_id") @db.ObjectId
          data Unsupported("JavaScriptCode")

          @@index([data], map: "data_1")
        }
    "#]];

    expected.assert_eq(res.datamodel());

    res.assert_warning_code(3);
    res.assert_warning(
        "These fields are not supported by the Prisma Client, because Prisma currently does not support their types.",
    );
}

#[test]
fn partial_indices_should_be_ignored() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti", "age": 9}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder()
            .unique(Some(false))
            .partial_filter_expression(Some(doc! { "age": { "$gt": 10 } }))
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
          id   String @id @default(auto()) @map("_id") @db.ObjectId
          age  Int
          name String
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn skip_index_pointing_to_non_existing_field() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti"}];

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
          id   String @id @default(auto()) @map("_id") @db.ObjectId
          name String
        }
    "#]];

    expected.assert_eq(res.datamodel());
}
