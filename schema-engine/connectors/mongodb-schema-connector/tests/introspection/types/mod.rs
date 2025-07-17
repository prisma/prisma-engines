mod composite;

use crate::introspection::test_api::*;
use bson::{Binary, Bson, DateTime, Decimal128, Timestamp, doc, oid::ObjectId};

#[test]
fn string() {
    let res = introspect(|db| async move {
        db.create_collection("A").await?;
        let collection = db.collection("A");

        let docs = vec![
            doc! {"first": "Musti", "second": "Naukio", "third": "MeowMeow"},
            doc! {"first": "MeowMeow", "second": null, "third": "Lol"},
            doc! {"first": "Lol", "second": "Bar"},
        ];

        collection.insert_many(docs).await.unwrap();

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id     String  @id @default(auto()) @map("_id") @db.ObjectId
          first  String
          second String?
          third  String?
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn double() {
    let res = introspect(|db| async move {
        db.create_collection("A").await?;
        let collection = db.collection("A");

        let docs = vec![
            doc! {"first": Bson::Double(1.23), "second": Bson::Double(2.23), "third": Bson::Double(3.33)},
            doc! {"first": Bson::Double(1.23), "second": null, "third": Bson::Double(3.33)},
            doc! {"first": Bson::Double(1.23), "second": Bson::Double(2.23)},
        ];

        collection.insert_many(docs).await.unwrap();

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id     String @id @default(auto()) @map("_id") @db.ObjectId
          first  Float
          second Float?
          third  Float?
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn bool() {
    let res = introspect(|db| async move {
        db.create_collection("A").await?;
        let collection = db.collection("A");

        let docs = vec![
            doc! {"first": true, "second": false, "third": false},
            doc! {"first": true, "second": null, "third": true},
            doc! {"first": true, "second": false},
        ];

        collection.insert_many(docs).await.unwrap();

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id     String   @id @default(auto()) @map("_id") @db.ObjectId
          first  Boolean
          second Boolean?
          third  Boolean?
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn int() {
    let res = introspect(|db| async move {
        db.create_collection("A").await?;
        let collection = db.collection("A");

        let docs = vec![
            doc! {"first": Bson::Int32(1), "second": Bson::Int32(1), "third": Bson::Int32(1)},
            doc! {"first": Bson::Int32(1), "second": null, "third": Bson::Int32(1)},
            doc! {"first": Bson::Int32(1), "second": Bson::Int32(1)},
        ];

        collection.insert_many(docs).await.unwrap();

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id     String @id @default(auto()) @map("_id") @db.ObjectId
          first  Int
          second Int?
          third  Int?
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn bigint() {
    let res = introspect(|db| async move {
        db.create_collection("A").await?;
        let collection = db.collection("A");

        let docs = vec![
            doc! {"first": Bson::Int64(1), "second": Bson::Int64(1), "third": Bson::Int64(1)},
            doc! {"first": Bson::Int64(1), "second": null, "third": Bson::Int64(1)},
            doc! {"first": Bson::Int64(1), "second": Bson::Int64(1)},
        ];

        collection.insert_many(docs).await.unwrap();

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id     String  @id @default(auto()) @map("_id") @db.ObjectId
          first  BigInt
          second BigInt?
          third  BigInt?
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn timestamp() {
    let res = introspect(|db| async move {
        db.create_collection("A").await?;
        let collection = db.collection("A");

        let docs = vec![
            doc! {
                "first": Bson::Timestamp(Timestamp { time: 1234, increment: 1 }),
                "second": Bson::Timestamp(Timestamp { time: 1234, increment: 1 }),
                "third": Bson::Timestamp(Timestamp { time: 1234, increment: 1 })
            },
            doc! {
                "first": Bson::Timestamp(Timestamp { time: 1234, increment: 1}),
                "second": null,
                "third": Bson::Timestamp(Timestamp { time: 1234, increment: 1}),
            },
            doc! {
                "first": Bson::Timestamp(Timestamp { time: 1234, increment: 1}),
                "second": Bson::Timestamp(Timestamp { time: 1234, increment: 1 }),
            },
        ];

        collection.insert_many(docs).await.unwrap();

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id     String    @id @default(auto()) @map("_id") @db.ObjectId
          first  DateTime
          second DateTime?
          third  DateTime?
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn binary() {
    let res = introspect(|db| async move {
        db.create_collection("A").await?;
        let collection = db.collection("A");

        let bin = Binary {
            bytes: b"deadbeef".to_vec(),
            subtype: mongodb::bson::spec::BinarySubtype::Generic,
        };

        let docs = vec![
            doc! {
                "first": bin.clone(),
                "second": bin.clone(),
                "third": bin.clone(),
            },
            doc! {
                "first": bin.clone(),
                "second": null,
                "third": bin.clone(),
            },
            doc! {
                "first": bin.clone(),
                "second": bin.clone(),
            },
        ];

        collection.insert_many(docs).await.unwrap();

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id     String @id @default(auto()) @map("_id") @db.ObjectId
          first  Bytes
          second Bytes?
          third  Bytes?
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn object_id() {
    let res = introspect(|db| async move {
        db.create_collection("A").await?;
        let collection = db.collection("A");

        let docs = vec![
            doc! {
                "first": ObjectId::new(),
                "second": ObjectId::new(),
                "third": ObjectId::new(),
            },
            doc! {
                "first": ObjectId::new(),
                "second": null,
                "third": ObjectId::new(),
            },
            doc! {
                "first": ObjectId::new(),
                "second": ObjectId::new(),
            },
        ];

        collection.insert_many(docs).await.unwrap();

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id     String  @id @default(auto()) @map("_id") @db.ObjectId
          first  String  @db.ObjectId
          second String? @db.ObjectId
          third  String? @db.ObjectId
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn date() {
    let res = introspect(|db| async move {
        db.create_collection("A").await?;
        let collection = db.collection("A");

        let docs = vec![
            doc! {
                "first": Bson::DateTime(DateTime::now()),
                "second": Bson::DateTime(DateTime::now()),
                "third": Bson::DateTime(DateTime::now()),
            },
            doc! {
                "first": Bson::DateTime(DateTime::now()),
                "second": null,
                "third": Bson::DateTime(DateTime::now()),
            },
            doc! {
                "first": Bson::DateTime(DateTime::now()),
                "second": Bson::DateTime(DateTime::now()),
            },
        ];

        collection.insert_many(docs).await.unwrap();

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id     String    @id @default(auto()) @map("_id") @db.ObjectId
          first  DateTime  @db.Date
          second DateTime? @db.Date
          third  DateTime? @db.Date
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn decimal() {
    let res = introspect(|db| async move {
        db.create_collection("A").await?;
        let collection = db.collection("A");

        let docs = vec![
            doc! {
                "first": Bson::Decimal128(Decimal128::from_bytes([0u8; 16])),
                "second": Bson::Decimal128(Decimal128::from_bytes([0u8; 16])),
                "third": Bson::Decimal128(Decimal128::from_bytes([0u8; 16])),
            },
            doc! {
                "first": Bson::Decimal128(Decimal128::from_bytes([0u8; 16])),
                "second": null,
                "third": Bson::Decimal128(Decimal128::from_bytes([0u8; 16])),
            },
            doc! {
                "first": Bson::Decimal128(Decimal128::from_bytes([0u8; 16])),
                "second": Bson::Decimal128(Decimal128::from_bytes([0u8; 16])),
            },
        ];

        collection.insert_many(docs).await.unwrap();

        Ok(())
    });

    let expected = expect![[r#"
      model A {
        id     String                     @id @default(auto()) @map("_id") @db.ObjectId
        first  Unsupported("Decimal128")
        second Unsupported("Decimal128")?
        third  Unsupported("Decimal128")?
      }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn array() {
    let res = introspect(|db| async move {
        db.create_collection("A").await?;
        let collection = db.collection("A");

        let docs = vec![
            doc! {
                "first": Bson::Array(vec![Bson::Int32(1)]),
                "second": Bson::Array(vec![Bson::Int32(1)]),
                "third": Bson::Array(vec![Bson::Int32(1)]),
            },
            doc! {
                "first": Bson::Array(vec![Bson::Int32(1)]),
                "second": null,
                "third": Bson::Array(vec![Bson::Int32(1)]),
            },
            doc! {
                "first": Bson::Array(vec![Bson::Int32(1)]),
                "second": Bson::Array(Vec::new()),
            },
        ];

        collection.insert_many(docs).await.unwrap();

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id     String @id @default(auto()) @map("_id") @db.ObjectId
          first  Int[]
          second Int[]
          third  Int[]
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn deep_array() {
    let res = introspect(|db| async move {
        db.create_collection("A").await?;
        let collection = db.collection("A");

        let docs = vec![doc! {
            "first": Bson::Array(vec![Bson::Array(vec![Bson::Int32(1)])]),
        }];

        collection.insert_many(docs).await.unwrap();

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id    String @id @default(auto()) @map("_id") @db.ObjectId
          first Json
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn empty_arrays() {
    let res = introspect(|db| async move {
        db.create_collection("A").await?;
        let collection = db.collection("A");

        collection
            .insert_one(doc! { "data": Bson::Array(Vec::new()) })
            .await
            .unwrap();

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id   String @id @default(auto()) @map("_id") @db.ObjectId
          /// Could not determine type: the field only had null or empty values in the sample set.
          data Json?
        }
    "#]];

    expected.assert_eq(res.datamodel());

    let expect = expect![[r#"
        *** WARNING ***

        Could not determine the types for the following fields:
          - Model: "A", field: "data"
    "#]];

    res.expect_warnings(&expect);
}

#[test]
fn unknown_types() {
    let res = introspect(|db| async move {
        db.create_collection("A").await?;
        let collection = db.collection("A");

        collection.insert_one(doc! { "data": Bson::Null }).await.unwrap();

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id   String @id @default(auto()) @map("_id") @db.ObjectId
          /// Could not determine type: the field only had null or empty values in the sample set.
          data Json?
        }
    "#]];

    expected.assert_eq(res.datamodel());

    let expect = expect![[r#"
        *** WARNING ***

        Could not determine the types for the following fields:
          - Model: "A", field: "data"
    "#]];

    res.expect_warnings(&expect);
}
