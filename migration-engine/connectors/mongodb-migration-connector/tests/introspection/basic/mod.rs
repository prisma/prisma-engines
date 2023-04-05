use crate::introspection::test_api::*;
use mongodb::bson::doc;

#[test]
fn empty_collection() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id String @id @default(auto()) @map("_id") @db.ObjectId
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn integer_id() {
    let res = introspect(|db| async move {
        let collection = db.collection("A");
        collection.insert_one(doc! { "_id": 12345 }, None).await.unwrap();

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id Int @id @map("_id")
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn multiple_collections_with_data() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"first": "Musti"}];

        collection.insert_many(docs, None).await.unwrap();

        db.create_collection("B", None).await?;
        let collection = db.collection("B");
        let docs = vec![doc! {"second": "Naukio"}];

        collection.insert_many(docs, None).await.unwrap();

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id    String @id @default(auto()) @map("_id") @db.ObjectId
          first String
        }

        model B {
          id     String @id @default(auto()) @map("_id") @db.ObjectId
          second String
        }
    "#]];

    expected.assert_eq(res.datamodel());
}
