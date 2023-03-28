use crate::test_api::*;
use mongodb::bson::{doc, Bson, DateTime, Timestamp};
use serde_json::json;

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
          id  String @id @default(auto()) @map("_id") @db.ObjectId
          id_ Int    @map("id")
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn mixed_id_types() {
    let res = introspect(|db| async move {
        db.collection("A")
            .insert_many(
                vec![doc! { "_id": 12345 }, doc! { "_id": "foo" }, doc! { "foo": false }],
                None,
            )
            .await
            .unwrap();

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          /// Multiple data types found: String: 33.3%, String (ObjectId): 33.3%, Int: 33.3% out of 3 sampled entries
          id  Json     @id @map("_id")
          foo Boolean?
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn mixing_types() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");
        let docs = vec![doc! {"first": "Musti"}, doc! {"first": 1i32}, doc! {"first": null}];

        collection.insert_many(docs, None).await.unwrap();

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id    String @id @default(auto()) @map("_id") @db.ObjectId
          /// Multiple data types found: String: 50%, Int: 50% out of 3 sampled entries
          first Json?
        }
    "#]];

    expected.assert_eq(res.datamodel());

    res.assert_warning_code(101);
    res.assert_warning(
        "The following fields had data stored in multiple types. Either use Json or normalize data to the wanted type.",
    );

    res.assert_warning_affected(&json!([{
        "model": "A",
        "field": "first",
        "tpe": "Json",
    }]));
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
          id    String    @id @default(auto()) @map("_id") @db.ObjectId
          /// Multiple data types found: DateTime (Date): 50%, DateTime (Timestamp): 50% out of 3 sampled entries
          first DateTime?
        }
    "#]];

    expected.assert_eq(res.datamodel());

    res.assert_warning_affected(&json!([{
        "model": "A",
        "field": "first",
        "tpe": "DateTime (Timestamp)",
    }]));
}
