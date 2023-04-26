use crate::introspection::test_api::*;
use mongodb::{
    bson::{doc, Bson},
    options::IndexOptions,
    IndexModel,
};
use schema_connector::CompositeTypeDepth;

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

    "#]];

    expected.assert_eq(res.datamodel());

    let expect = expect![""];

    res.expect_warnings(&expect);
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

    "#]];

    expected.assert_eq(res.datamodel());

    let expect = expect![""];

    res.expect_warnings(&expect);
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

    "#]];

    expected.assert_eq(res.datamodel());

    let expect = expect![""];

    res.expect_warnings(&expect);
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

    "#]];

    expected.assert_eq(res.datamodel());

    let expect = expect![""];

    res.expect_warnings(&expect);
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

    "#]];

    expected.assert_eq(res.datamodel());

    let expect = expect![""];

    res.expect_warnings(&expect);
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

    "#]];

    expected.assert_eq(res.datamodel());

    let expected = expect![""];

    res.expect_warnings(&expected);
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

    "#]];

    expected.assert_eq(res.datamodel());

    let expect = expect![""];

    res.expect_warnings(&expect);
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

    "#]];

    expected.assert_eq(res.datamodel());

    let expect = expect![""];

    res.expect_warnings(&expect);
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

    "#]];

    expected.assert_eq(res.datamodel());

    let expect = expect![""];

    res.expect_warnings(&expect);
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

    "#]];

    expected.assert_eq(res.datamodel());

    let expect = expect![""];

    res.expect_warnings(&expect);
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

    "#]];

    expected.assert_eq(res.datamodel());

    let expect = expect![""];

    res.expect_warnings(&expect);
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

    "#]];

    expected.assert_eq(res.datamodel());

    let expect = expect![""];

    res.expect_warnings(&expect);
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

    "#]];

    expected.assert_eq(res.datamodel());

    let expect = expect![""];

    res.expect_warnings(&expect);
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

    "#]];

    expected.assert_eq(res.datamodel());

    let expect = expect![""];

    res.expect_warnings(&expect);
}

#[test]
fn composite_type_index_without_corresponding_data_should_not_crash() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection::<mongodb::bson::Document>("A");

        let model = IndexModel::builder().keys(doc! { "foo": 1 }).build();

        collection.create_index(model, None).await?;

        let model = IndexModel::builder().keys(doc! { "foo.bar": 1 }).build();

        collection.create_index(model, None).await?;

        let model = IndexModel::builder().keys(doc! { "foo.baz.quux": 1 }).build();

        collection.create_index(model, None).await?;

        Ok(())
    });

    let expected = expect![[r#"

    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn composite_type_index_with_non_composite_fields_in_the_middle_should_not_crash() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection::<mongodb::bson::Document>("A");

        let model = IndexModel::builder().keys(doc! { "a.b.c": 1 }).build();
        collection.create_index(model, None).await?;

        let docs = vec![doc! { "a": { "b": 1, "d": { "c": 1 } } }];
        collection.insert_many(docs, None).await.unwrap();

        Ok(())
    });

    let expected = expect![[r#"

    "#]];

    expected.assert_eq(res.datamodel());
}
