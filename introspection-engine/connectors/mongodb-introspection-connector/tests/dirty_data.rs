mod common;

use bson::{doc, Bson, DateTime, Timestamp};
use common::*;
use expect_test::expect;

#[test]
fn explicit_id_field() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"id": 1}];

        collection.insert_many(docs, None).await.unwrap();

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id String @id @default(dbgenerated()) @map("_id") @db.ObjectId
          id Int    @id
        }
    "#]];

    expected.assert_eq(res.datamodel());
    res.assert_warning("The given model has a field with the name `id`, that clashes with the primary key. Please rename either one of them before using the data model.");
}

#[test]
fn mixing_types() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"first": "Musti"}, doc! {"first": 1}, doc! {"first": null}];

        collection.insert_many(docs, None).await.unwrap();

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id    String @id @default(dbgenerated()) @map("_id") @db.ObjectId
          /// String: 50%, Int32: 50%
          first Int?
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn mixing_types_with_the_same_base_type() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");

        let docs = vec![
            doc! {"first": Bson::Timestamp(Timestamp { time: 1, increment: 1 })},
            doc! {"first": Bson::DateTime(DateTime::now())},
            doc! {"first": null},
        ];

        collection.insert_many(docs, None).await.unwrap();

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id    String    @id @default(dbgenerated()) @map("_id") @db.ObjectId
          /// Date: 50%, Timestamp: 50%
          first DateTime?
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn the_most_common_type_wins() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"first": "Musti"}, doc! {"first": "Naukio"}, doc! {"first": false}];

        collection.insert_many(docs, None).await.unwrap();

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id    String @id @default(dbgenerated()) @map("_id") @db.ObjectId
          /// String: 66.7%, Boolean: 33.3%
          first String
        }
    "#]];

    expected.assert_eq(res.datamodel());
}
