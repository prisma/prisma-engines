use crate::test_api::*;
use introspection_connector::CompositeTypeDepth;
use mongodb::{
    bson::{doc, Bson},
    options::IndexOptions,
    IndexModel,
};
use serde_json::json;

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
fn single_column_composite_index() {
    let res = introspect(|db| async move {
        db.create_collection("Cat", None).await?;
        let collection = db.collection("Cat");
        let docs = vec![doc! {"name": "Musti", "address": { "number": 27 } }];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "address.number": 1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type CatAddress {
          number Int
        }

        model Cat {
          id      String     @id @default(auto()) @map("_id") @db.ObjectId
          address CatAddress
          name    String

          @@index([address.number], map: "address.number_1")
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn single_column_composite_array_index() {
    let res = introspect(|db| async move {
        db.create_collection("Cat", None).await?;
        let collection = db.collection("Cat");
        let docs = vec![doc! {"name": "Musti", "addresses": [ { "number": 27 }, { "number": 28 } ] }];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "addresses.number": 1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type CatAddresses {
          number Int
        }

        model Cat {
          id        String         @id @default(auto()) @map("_id") @db.ObjectId
          addresses CatAddresses[]
          name      String

          @@index([addresses.number], map: "addresses.number_1")
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn single_column_deep_composite_index() {
    let res = introspect(|db| async move {
        db.create_collection("Cat", None).await?;
        let collection = db.collection("Cat");
        let docs = vec![doc! {"name": "Musti", "address": { "special": { "number": 27 } } }];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "address.special.number": 1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type CatAddress {
          special CatAddressSpecial
        }

        type CatAddressSpecial {
          number Int
        }

        model Cat {
          id      String     @id @default(auto()) @map("_id") @db.ObjectId
          address CatAddress
          name    String

          @@index([address.special.number], map: "address.special.number_1")
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
fn single_column_descending_composite_index() {
    let res = introspect(|db| async move {
        db.create_collection("Cat", None).await?;
        let collection = db.collection("Cat");
        let docs = vec![doc! {"name": "Musti", "address": { "number": 27 }}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "address.number": -1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type CatAddress {
          number Int
        }

        model Cat {
          id      String     @id @default(auto()) @map("_id") @db.ObjectId
          address CatAddress
          name    String

          @@index([address.number(sort: Desc)], map: "address.number_-1")
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
fn single_column_fulltext_composite_index() {
    let res = introspect(|db| async move {
        db.create_collection("Cat", None).await?;
        let collection = db.collection("Cat");
        let docs = vec![doc! {"name": "Musti", "address": { "street": "Meowallee" }}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "address.street": "text" })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type CatAddress {
          street String
        }

        model Cat {
          id      String     @id @default(auto()) @map("_id") @db.ObjectId
          address CatAddress
          name    String

          @@fulltext([address.street], map: "address.street_\"text\"")
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn single_array_column_fulltext_composite_index() {
    let res = introspect(|db| async move {
        db.create_collection("Cat", None).await?;
        let collection = db.collection("Cat");
        let docs =
            vec![doc! {"name": "Musti", "addresses": [ { "street": "Meowallee" }, { "street": "Purrstrasse" } ] }];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "addresses.street": "text" })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type CatAddresses {
          street String
        }

        model Cat {
          id        String         @id @default(auto()) @map("_id") @db.ObjectId
          addresses CatAddresses[]
          name      String

          @@fulltext([addresses.street], map: "addresses.street_\"text\"")
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
fn multi_column_fulltext_composite_index() {
    let res = introspect(|db| async move {
        db.create_collection("Cat", None).await?;
        let collection = db.collection("Cat");
        let docs = vec![doc! {"name": "Musti", "address": { "street": "Meowallee", "city": "Derplin" } }];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "address.street": "text", "address.city": "text" })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type CatAddress {
          city   String
          street String
        }

        model Cat {
          id      String     @id @default(auto()) @map("_id") @db.ObjectId
          address CatAddress
          name    String

          @@fulltext([address.city, address.street], map: "address.street_\"text\"_address.city_\"text\"")
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
fn multi_column_fulltext_composite_index_with_desc_in_end() {
    let res = introspect(|db| async move {
        db.create_collection("Cat", None).await?;
        let collection = db.collection("Cat");
        let docs = vec![doc! {"name": "Musti", "address": { "street": "Meowallee", "number": 69 }}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "name": "text", "address.street": "text", "address.number": -1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type CatAddress {
          number Int
          street String
        }

        model Cat {
          id      String     @id @default(auto()) @map("_id") @db.ObjectId
          address CatAddress
          name    String

          @@fulltext([address.street, name, address.number(sort: Desc)], map: "name_\"text\"_address.street_\"text\"_address.number_-1")
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
fn multi_column_fulltext_composite_index_with_desc_in_beginning() {
    let res = introspect(|db| async move {
        db.create_collection("Cat", None).await?;
        let collection = db.collection("Cat");
        let docs = vec![doc! {"name": "Musti", "address": { "street": "Meowallee", "number": 69 }}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "address.number": -1, "address.street": "text", "name": "text" })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type CatAddress {
          number Int
          street String
        }

        model Cat {
          id      String     @id @default(auto()) @map("_id") @db.ObjectId
          address CatAddress
          name    String

          @@fulltext([address.number(sort: Desc), address.street, name], map: "address.number_-1_address.street_\"text\"_name_\"text\"")
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

    let res = introspect_features(depth, Default::default(), |db| async move {
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
fn fultext_composite_index_without_preview_flag() {
    let depth = CompositeTypeDepth::Infinite;

    let res = introspect_features(depth, Default::default(), |db| async move {
        db.create_collection("Cat", None).await?;
        let collection = db.collection("Cat");
        let docs = vec![doc! {"name": "Musti", "address": { "street": "Meowallee" } }];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "address.street": "text" })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type CatAddress {
          street String
        }

        model Cat {
          id      String     @id @default(auto()) @map("_id") @db.ObjectId
          address CatAddress
          name    String
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
fn composite_index_pointing_to_a_renamed_field() {
    let res = introspect(|db| async move {
        db.create_collection("Cat", None).await?;
        let collection = db.collection("Cat");
        let docs = vec![doc! { "name": "Musti", "info": { "_age": 9} }];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "info._age": 1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type CatInfo {
          age Int @map("_age")
        }

        model Cat {
          id   String  @id @default(auto()) @map("_id") @db.ObjectId
          info CatInfo
          name String

          @@index([info.age], map: "info._age_1")
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
fn single_column_normal_composite_index_default_name() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti", "info": { "age": 9} }];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder()
            .unique(Some(false))
            .name(Some("A_info_age_idx".to_string()))
            .build();

        let model = IndexModel::builder()
            .keys(doc! { "info.age": 1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type AInfo {
          age Int
        }

        model A {
          id   String @id @default(auto()) @map("_id") @db.ObjectId
          info AInfo
          name String

          @@index([info.age])
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
fn single_column_unique_composite_index() {
    let res = introspect(|db| async move {
        db.create_collection("Cat", None).await?;
        let collection = db.collection("Cat");
        let docs = vec![doc! {"name": "Musti", "info": { "age": 9 } }];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(true)).build();

        let model = IndexModel::builder()
            .keys(doc! { "info.age": 1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type CatInfo {
          age Int
        }

        model Cat {
          id   String  @id @default(auto()) @map("_id") @db.ObjectId
          info CatInfo
          name String

          @@unique([info.age], map: "info.age_1")
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn single_array_column_unique_composite_index() {
    let res = introspect(|db| async move {
        db.create_collection("Cat", None).await?;
        let collection = db.collection("Cat");
        let docs = vec![doc! {"name": "Musti", "infos": [ { "age": 9 }, { "age": 10 } ] }];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(true)).build();

        let model = IndexModel::builder()
            .keys(doc! { "infos.age": 1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type CatInfos {
          age Int
        }

        model Cat {
          id    String     @id @default(auto()) @map("_id") @db.ObjectId
          infos CatInfos[]
          name  String

          @@unique([infos.age], map: "infos.age_1")
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
fn single_column_unique_composite_index_default_name() {
    let res = introspect(|db| async move {
        db.create_collection("Cat", None).await?;
        let collection = db.collection("Cat");
        let docs = vec![doc! {"name": "Musti", "info": { "age": 9 } }];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder()
            .unique(Some(true))
            .name(Some("Cat_info_age_key".to_string()))
            .build();

        let model = IndexModel::builder()
            .keys(doc! { "info.age": 1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type CatInfo {
          age Int
        }

        model Cat {
          id   String  @id @default(auto()) @map("_id") @db.ObjectId
          info CatInfo
          name String

          @@unique([info.age])
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
fn multi_column_unique_composite_index() {
    let res = introspect(|db| async move {
        db.create_collection("Cat", None).await?;
        let collection = db.collection("Cat");
        let docs = vec![doc! {"name": "Musti", "info": { "age": 9 } }];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(true)).build();

        let model = IndexModel::builder()
            .keys(doc! { "name": 1, "info.age": -1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type CatInfo {
          age Int
        }

        model Cat {
          id   String  @id @default(auto()) @map("_id") @db.ObjectId
          info CatInfo
          name String

          @@unique([name, info.age(sort: Desc)], map: "name_1_info.age_-1")
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
fn partial_composite_indices_should_be_ignored() {
    let res = introspect(|db| async move {
        db.create_collection("Cat", None).await?;
        let collection = db.collection("Cat");
        let docs = vec![doc! {"name": "Musti", "info": { "age": 9 }}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder()
            .unique(Some(false))
            .partial_filter_expression(Some(doc! { "info.age": { "$gt": 10 } }))
            .build();

        let model = IndexModel::builder()
            .keys(doc! { "info.age": 1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type CatInfo {
          age Int
        }

        model Cat {
          id   String  @id @default(auto()) @map("_id") @db.ObjectId
          info CatInfo
          name String
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn index_pointing_to_non_existing_field_should_add_the_field() {
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
          /// Field referred in an index, but found no data to define the type.
          age  Json?

          @@index([age], map: "age_1")
        }
    "#]];

    expected.assert_eq(res.datamodel());

    res.assert_warning("Could not determine the types for the following fields.");
    res.assert_warning_code(103);
    res.assert_warning_affected(&json!([{
        "model": "A",
        "field": "age"
    }]));
}

#[test]
fn index_pointing_to_non_existing_composite_field_should_add_the_field_and_type() {
    let res = introspect(|db| async move {
        db.create_collection("Cat", None).await?;
        let collection = db.collection("Cat");
        let docs = vec![doc! {"name": "Musti"}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "info.age": 1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type CatInfo {
          /// Field referred in an index, but found no data to define the type.
          age Json?
        }

        model Cat {
          id   String   @id @default(auto()) @map("_id") @db.ObjectId
          name String
          /// Field referred in an index, but found no data to define the type.
          info CatInfo?

          @@index([info.age], map: "info.age_1")
        }
    "#]];

    expected.assert_eq(res.datamodel());

    res.assert_warning("Could not determine the types for the following fields.");
    res.assert_warning_code(103);
    res.assert_warning_affected(&json!([{
        "compositeType": "CatInfo",
        "field": "age"
    }]));
}

#[test]
fn deep_index_pointing_to_non_existing_composite_field_should_add_the_field_and_type() {
    let res = introspect(|db| async move {
        db.create_collection("Cat", None).await?;
        let collection = db.collection("Cat");
        let docs = vec![doc! {"name": "Musti"}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "info.specific.age": 1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type CatInfo {
          /// Field referred in an index, but found no data to define the type.
          specific CatInfoSpecific?
        }

        type CatInfoSpecific {
          /// Field referred in an index, but found no data to define the type.
          age Json?
        }

        model Cat {
          id   String   @id @default(auto()) @map("_id") @db.ObjectId
          name String
          /// Field referred in an index, but found no data to define the type.
          info CatInfo?

          @@index([info.specific.age], map: "info.specific.age_1")
        }
    "#]];

    expected.assert_eq(res.datamodel());

    res.assert_warning("Could not determine the types for the following fields.");
    res.assert_warning_code(103);
    res.assert_warning_affected(&json!([{
        "compositeType": "CatInfoSpecific",
        "field": "age"
    }]));
}

#[test]
fn index_pointing_to_mapped_non_existing_field_should_add_the_mapped_field() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti"}];

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
          name String
          /// Field referred in an index, but found no data to define the type.
          age  Json?  @map("_age")

          @@index([age], map: "_age_1")
        }
    "#]];

    expected.assert_eq(res.datamodel());

    res.assert_warning("Could not determine the types for the following fields.");
    res.assert_warning_code(103);
    res.assert_warning_affected(&json!([{
        "model": "A",
        "field": "_age"
    }]));
}

#[test]
fn composite_index_pointing_to_mapped_non_existing_field_should_add_the_mapped_field() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti"}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "info._age": 1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type AInfo {
          /// Field referred in an index, but found no data to define the type.
          age Json? @map("_age")
        }

        model A {
          id   String @id @default(auto()) @map("_id") @db.ObjectId
          name String
          /// Field referred in an index, but found no data to define the type.
          info AInfo?

          @@index([info.age], map: "info._age_1")
        }
    "#]];

    expected.assert_eq(res.datamodel());

    res.assert_warning("Could not determine the types for the following fields.");
    res.assert_warning_code(103);
    res.assert_warning_affected(&json!([{
        "compositeType": "AInfo",
        "field": "age"
    }]));
}

#[test]
fn compound_index_pointing_to_non_existing_field_should_add_the_field() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti"}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "age": 1, "play": 1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id   String @id @default(auto()) @map("_id") @db.ObjectId
          name String
          /// Field referred in an index, but found no data to define the type.
          age  Json?
          /// Field referred in an index, but found no data to define the type.
          play Json?

          @@index([age, play], map: "age_1_play_1")
        }
    "#]];

    expected.assert_eq(res.datamodel());

    res.assert_warning("Could not determine the types for the following fields.");
    res.assert_warning_code(103);
    res.assert_warning_affected(&json!([
        {
            "model": "A",
            "field": "age"
        },
        {
            "model": "A",
            "field": "play"
        },
    ]));
}

#[test]
fn composite_index_with_one_existing_field_should_add_missing_stuff_only() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti", "info": { "age": 9 } }];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "info.age": 1, "info.play": 1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type AInfo {
          age  Int
          /// Field referred in an index, but found no data to define the type.
          play Json?
        }

        model A {
          id   String @id @default(auto()) @map("_id") @db.ObjectId
          info AInfo
          name String

          @@index([info.age, info.play], map: "info.age_1_info.play_1")
        }
    "#]];

    expected.assert_eq(res.datamodel());

    res.assert_warning("Could not determine the types for the following fields.");
    res.assert_warning_code(103);
    res.assert_warning_affected(&json!([
        {
            "compositeType": "AInfo",
            "field": "play"
        },
    ]));
}

#[test]
fn deep_composite_index_with_one_existing_field_should_add_missing_stuff_only() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti", "info": { "age": 9 } }];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "info.age": 1, "info.special.play": 1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type AInfo {
          age     Int
          /// Field referred in an index, but found no data to define the type.
          special AInfoSpecial?
        }

        type AInfoSpecial {
          /// Field referred in an index, but found no data to define the type.
          play Json?
        }

        model A {
          id   String @id @default(auto()) @map("_id") @db.ObjectId
          info AInfo
          name String

          @@index([info.age, info.special.play], map: "info.age_1_info.special.play_1")
        }
    "#]];

    expected.assert_eq(res.datamodel());

    res.assert_warning("Could not determine the types for the following fields.");
    res.assert_warning_code(103);
    res.assert_warning_affected(&json!([
        {
            "compositeType": "AInfoSpecial",
            "field": "play"
        },
    ]));
}

#[test]
fn deep_composite_index_with_one_existing_field_should_add_missing_stuff_only_2() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti", "info": { "special": { "age": 9 } } }];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "info.special.age": 1, "info.special.play": 1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type AInfo {
          special AInfoSpecial
        }

        type AInfoSpecial {
          age  Int
          /// Field referred in an index, but found no data to define the type.
          play Json?
        }

        model A {
          id   String @id @default(auto()) @map("_id") @db.ObjectId
          info AInfo
          name String

          @@index([info.special.age, info.special.play], map: "info.special.age_1_info.special.play_1")
        }
    "#]];

    expected.assert_eq(res.datamodel());

    res.assert_warning("Could not determine the types for the following fields.");
    res.assert_warning_code(103);
    res.assert_warning_affected(&json!([
        {
            "compositeType": "AInfoSpecial",
            "field": "play"
        },
    ]));
}

#[test]
fn deep_composite_index_should_add_missing_stuff_in_different_layers() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! { "name": "Musti" }];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "info.special.age": 1, "info.play": 1 })
            .options(Some(options))
            .build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type AInfo {
          /// Field referred in an index, but found no data to define the type.
          special AInfoSpecial?
          /// Field referred in an index, but found no data to define the type.
          play    Json?
        }

        type AInfoSpecial {
          /// Field referred in an index, but found no data to define the type.
          age Json?
        }

        model A {
          id   String @id @default(auto()) @map("_id") @db.ObjectId
          name String
          /// Field referred in an index, but found no data to define the type.
          info AInfo?

          @@index([info.special.age, info.play], map: "info.special.age_1_info.play_1")
        }
    "#]];

    expected.assert_eq(res.datamodel());

    res.assert_warning("Could not determine the types for the following fields.");
    res.assert_warning_code(103);
    res.assert_warning_affected(&json!([
        {
            "compositeType": "AInfoSpecial",
            "field": "age"
        },
        {
            "compositeType": "AInfo",
            "field": "play"
        },
    ]));
}

#[test]
fn compound_index_with_one_existing_field_pointing_to_non_existing_field_should_add_the_field() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti", "age": 9}];

        collection.insert_many(docs, None).await.unwrap();

        let options = IndexOptions::builder().unique(Some(false)).build();

        let model = IndexModel::builder()
            .keys(doc! { "age": 1, "play": 1 })
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
          /// Field referred in an index, but found no data to define the type.
          play Json?

          @@index([age, play], map: "age_1_play_1")
        }
    "#]];

    expected.assert_eq(res.datamodel());

    res.assert_warning("Could not determine the types for the following fields.");
    res.assert_warning_code(103);
    res.assert_warning_affected(&json!([
        {
            "model": "A",
            "field": "play"
        },
    ]));
}

#[test]
fn unique_index_pointing_to_non_existing_field_should_add_the_field() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti"}];

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
          name String
          /// Field referred in an index, but found no data to define the type.
          age  Json?  @unique(map: "age_1")
        }
    "#]];

    expected.assert_eq(res.datamodel());

    res.assert_warning("Could not determine the types for the following fields.");
    res.assert_warning_code(103);
    res.assert_warning_affected(&json!([{
        "model": "A",
        "field": "age"
    }]));
}

#[test]
fn fulltext_index_pointing_to_non_existing_field_should_add_the_field() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"name": "Musti"}];

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
          name String
          /// Field referred in an index, but found no data to define the type.
          age  Json?  @unique(map: "age_1")
        }
    "#]];

    expected.assert_eq(res.datamodel());

    res.assert_warning("Could not determine the types for the following fields.");
    res.assert_warning_code(103);
    res.assert_warning_affected(&json!([{
        "model": "A",
        "field": "age"
    }]));
}
