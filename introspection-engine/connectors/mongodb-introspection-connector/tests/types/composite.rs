use crate::test_api::*;
use bson::Bson;
use introspection_connector::CompositeTypeDepth;

#[test]
fn singular() {
    let res = introspect(|db| async move {
        let docs = vec![
            doc! { "name": "Musti", "address": { "street": "Meowstrasse", "number": 123 }},
            doc! { "name": "Naukio", "address": { "street": "Meowstrasse", "number": 123, "knock": true }},
        ];

        db.collection("Cat").insert_many(docs, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type CatAddress {
          knock  Boolean?
          number Int
          street String
        }

        model Cat {
          id      String     @id @default(dbgenerated()) @map("_id") @db.ObjectId
          address CatAddress
          name    String
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn dirty_data() {
    let res = introspect(|db| async move {
        let docs = vec![
            doc! { "name": "Musti", "address": { "street": "Meowstrasse", "number": 123 }},
            doc! { "name": "Naukio", "address": { "street": "Meowstrasse", "number": "123" }},
            doc! { "name": "Bob", "address": { "street": "Kantstrasse", "number": "123" }},
        ];

        db.collection("Cat").insert_many(docs, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type CatAddress {
          /// Multiple data types found: String: 66.7%, Int32: 33.3% out of 3 sampled entries
          number String
          street String
        }

        model Cat {
          id      String     @id @default(dbgenerated()) @map("_id") @db.ObjectId
          address CatAddress
          name    String
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn array() {
    let res = introspect(|db| async move {
        let docs = vec![doc! { "title": "Test", "posts": [
            { "title": "Test", "published": false },
            { "title": "Hello, world!", "content": "Like, whatever...", "published": true },
        ]}];

        db.collection("Blog").insert_many(docs, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type BlogPosts {
          content   String?
          published Boolean
          title     String
        }

        model Blog {
          id    String      @id @default(dbgenerated()) @map("_id") @db.ObjectId
          posts BlogPosts[]
          title String
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn deep_array() {
    let res = introspect(|db| async move {
        let docs = vec![doc! { "title": "Test", "posts": [
            [{ "title": "Test", "published": false }],
            [{ "title": "Hello, world!", "content": "Like, whatever...", "published": true }],
        ]}];

        db.collection("Blog").insert_many(docs, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        model Blog {
          id    String @id @default(dbgenerated()) @map("_id") @db.ObjectId
          posts Json[]
          title String
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn nullability() {
    let res = introspect(|db| async move {
        let collection = db.collection("Model");

        let docs = vec![
            doc! {"first": {"foo": 1}, "second": {"foo": 1}, "third": {"foo": 1}},
            doc! {"first": {"foo": 1}, "second": null, "third": {"foo": 1}},
            doc! {"first": {"foo": 1}, "second": {"foo": 1}},
        ];

        collection.insert_many(docs, None).await.unwrap();

        Ok(())
    });

    let expected = expect![[r#"
        type ModelFirst {
          foo Int
        }

        type ModelSecond {
          foo Int
        }

        type ModelThird {
          foo Int
        }

        model Model {
          id     String       @id @default(dbgenerated()) @map("_id") @db.ObjectId
          first  ModelFirst
          second ModelSecond?
          third  ModelThird?
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn unsupported() {
    let res = introspect(|db| async move {
        let docs =
            vec![doc! { "dataType": "Code", "data": { "code": Bson::JavaScriptCode("let a = 1 + 1;".to_string()) }}];

        db.collection("FrontendEngineerWritesBackendCode")
            .insert_many(docs, None)
            .await?;

        Ok(())
    });

    let expected = expect![[r#"
        type FrontendEngineerWritesBackendCodeData {
          code Unsupported("JavaScriptCode")
        }

        model FrontendEngineerWritesBackendCode {
          id       String                                @id @default(dbgenerated()) @map("_id") @db.ObjectId
          data     FrontendEngineerWritesBackendCodeData
          dataType String
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn underscores_in_names() {
    let res = introspect(|db| async move {
        let docs = vec![doc! { "name": "Musti", "home_address": { "street": "Meowstrasse", "number": 123 }}];
        db.collection("Cat").insert_many(docs, None).await?;
        Ok(())
    });

    let expected = expect![[r#"
        type CatHomeAddress {
          number Int
          street String
        }

        model Cat {
          id           String         @id @default(dbgenerated()) @map("_id") @db.ObjectId
          home_address CatHomeAddress
          name         String
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn depth_none() {
    let res = introspect_depth(CompositeTypeDepth::None, |db| async move {
        let docs = vec![doc! { "name": "Musti", "home_address": { "street": "Meowstrasse", "number": 123 }}];
        db.collection("Cat").insert_many(docs, None).await?;
        Ok(())
    });

    let expected = expect![[r#"
        model Cat {
          id           String @id @default(dbgenerated()) @map("_id") @db.ObjectId
          home_address Json
          name         String
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn depth_none_level_1_array() {
    let res = introspect_depth(CompositeTypeDepth::None, |db| async move {
        let docs = vec![doc! { "name": "Musti", "home_address": [{ "street": "Meowstrasse", "number": 123 }]}];
        db.collection("Cat").insert_many(docs, None).await?;
        Ok(())
    });

    let expected = expect![[r#"
        model Cat {
          id           String @id @default(dbgenerated()) @map("_id") @db.ObjectId
          home_address Json[]
          name         String
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn depth_1_level_1() {
    let res = introspect_depth(CompositeTypeDepth::Level(1), |db| async move {
        let docs = vec![doc! { "name": "Musti", "home_address": { "street": "Meowstrasse", "number": 123 }}];
        db.collection("Cat").insert_many(docs, None).await?;
        Ok(())
    });

    let expected = expect![[r#"
        type CatHomeAddress {
          number Int
          street String
        }

        model Cat {
          id           String         @id @default(dbgenerated()) @map("_id") @db.ObjectId
          home_address CatHomeAddress
          name         String
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn depth_1_level_2() {
    let res = introspect_depth(CompositeTypeDepth::Level(1), |db| async move {
        let docs = vec![
            doc! { "name": "Musti", "home_address": { "street": "Meowstrasse", "number": 123, "data": { "something": "other" } } },
        ];
        db.collection("Cat").insert_many(docs, None).await?;
        Ok(())
    });

    let expected = expect![[r#"
        type CatHomeAddress {
          data   Json
          number Int
          street String
        }

        model Cat {
          id           String         @id @default(dbgenerated()) @map("_id") @db.ObjectId
          home_address CatHomeAddress
          name         String
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn depth_1_level_2_array() {
    let res = introspect_depth(CompositeTypeDepth::Level(1), |db| async move {
        let docs = vec![
            doc! { "name": "Musti", "home_address": [{ "street": "Meowstrasse", "number": 123, "data": [{ "something": "other" }] }] },
        ];
        db.collection("Cat").insert_many(docs, None).await?;
        Ok(())
    });

    let expected = expect![[r#"
        type CatHomeAddress {
          data   Json[]
          number Int
          street String
        }

        model Cat {
          id           String           @id @default(dbgenerated()) @map("_id") @db.ObjectId
          home_address CatHomeAddress[]
          name         String
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn depth_2_level_2_array() {
    let res = introspect_depth(CompositeTypeDepth::Level(2), |db| async move {
        let docs = vec![
            doc! { "name": "Musti", "home_address": [{ "street": "Meowstrasse", "number": 123, "data": [{ "something": "other" }] }] },
        ];
        db.collection("Cat").insert_many(docs, None).await?;
        Ok(())
    });

    let expected = expect![[r#"
        type CatHomeAddress {
          data   CatHomeAddressData[]
          number Int
          street String
        }

        type CatHomeAddressData {
          something String
        }

        model Cat {
          id           String           @id @default(dbgenerated()) @map("_id") @db.ObjectId
          home_address CatHomeAddress[]
          name         String
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn name_clashes() {
    let res = introspect(|db| async move {
        let docs = vec![doc! { "name": "Musti", "address": { "street": "Meowstrasse", "number": 123 }}];
        db.collection("Cat").insert_many(docs, None).await?;

        let docs = vec![doc! { "knock": false, "number": 420, "street": "Meowstrasse" }];
        db.collection("CatAddress").insert_many(docs, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        type CatAddress_ {
          number Int
          street String
        }

        model Cat {
          id      String      @id @default(dbgenerated()) @map("_id") @db.ObjectId
          address CatAddress_
          name    String
        }

        model CatAddress {
          id     String  @id @default(dbgenerated()) @map("_id") @db.ObjectId
          knock  Boolean
          number Int
          street String
        }
    "#]];

    expected.assert_eq(res.datamodel());
}
