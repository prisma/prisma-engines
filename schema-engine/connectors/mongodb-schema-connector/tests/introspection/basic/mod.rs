use crate::introspection::test_api::*;
use mongodb::{bson::doc, options::CreateCollectionOptions};

#[test]
fn empty_collection() {
    let res = introspect(|db| async move {
        db.create_collection("A").await?;

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
        collection.insert_one(doc! { "_id": 12345 }).await.unwrap();

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
        db.create_collection("A").await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"first": "Musti"}];

        collection.insert_many(docs).await.unwrap();

        db.create_collection("B").await?;
        let collection = db.collection("B");
        let docs = vec![doc! {"second": "Naukio"}];

        collection.insert_many(docs).await.unwrap();

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

#[test]
fn collection_with_json_schema() {
    let res = introspect(|db| async move {
        db.create_collection("A")
            .with_options(
                CreateCollectionOptions::builder()
                    .validator(Some(mongodb::bson::doc! {
                        "$jsonSchema": {
                            "bsonType": "object",
                            "title": "Student Object Validation",
                            "required": [ "address", "major", "name" ],
                            "properties": {
                               "name": {
                                  "bsonType": "string",
                                  "description": "'name' must be a string and is required"
                               },
                               "gpa": {
                                  "bsonType": [ "double" ],
                                  "description": "'gpa' must be a double if the field exists"
                               }
                            }
                         }
                    }))
                    .build(),
            )
            .await?;

        Ok(())
    });

    let expected_warning = expect![[r#"
        *** WARNING ***

        The following models have a JSON Schema defined in the database, which is not yet fully supported. Read more: https://pris.ly/d/mongodb-json-schema
          - "A"
    "#]];

    res.expect_warnings(&expected_warning);

    let expected_doc = expect![[r#"
        /// This collection uses a JSON Schema defined in the database, which requires additional setup for migrations. Visit https://pris.ly/d/mongodb-json-schema for more info.
        model A {
          id String @id @default(auto()) @map("_id") @db.ObjectId
        }
    "#]];
    expected_doc.assert_eq(res.datamodel());
}

#[test]
fn capped_collection() {
    let res = introspect(|db| async move {
        db.create_collection("A")
            .with_options(
                CreateCollectionOptions::builder()
                    .capped(Some(true))
                    .size(Some(1024))
                    .build(),
            )
            .await?;

        Ok(())
    });

    let expected_warning = expect![[r#"
        *** WARNING ***

        The following models are capped collections, which are not yet fully supported. Read more: https://pris.ly/d/mongodb-capped-collections
          - "A"
    "#]];
    res.expect_warnings(&expected_warning);

    let expected_doc = expect![[r#"
        /// This model is a capped collection, which is not yet fully supported. Read more: https://pris.ly/d/mongodb-capped-collections
        model A {
          id String @id @default(auto()) @map("_id") @db.ObjectId
        }
    "#]];
    expected_doc.assert_eq(res.datamodel());
}
