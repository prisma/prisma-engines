mod common;

use bson::doc;
use common::*;
use expect_test::expect;

#[test]
fn smoke() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        let collection = db.collection("A");

        let document = doc! {"name": "Musti", "age": 9};

        collection.insert_one(document, None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id   String @id @default(dbgenerated()) @map("_id") @db.ObjectId
          age  Int
          name String
        }
    "#]];

    expected.assert_eq(&res.datamodel());
}
