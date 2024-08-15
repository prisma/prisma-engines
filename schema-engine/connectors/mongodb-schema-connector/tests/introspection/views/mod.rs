use crate::introspection::test_api::*;
use mongodb::{bson::doc, options::CreateCollectionOptions};

#[test]
fn collection_with_view() {
    let res = introspect(|db| async move {
        db.create_collection("A").await?;
        db.create_collection("myView".to_string())
            .with_options(
                CreateCollectionOptions::builder()
                    .view_on("A".to_string())
                    .pipeline(vec![doc! {
                        "$lookup": {
                            "from": "A",
                            "localField": "first",
                            "foreignField": "_id",
                            "as": "someid"
                        },
                    }])
                    .build(),
            )
            .await?;

        Ok(())
    });

    let expected_warning = expect![""];
    res.expect_warnings(&expected_warning);

    let expected_doc = expect![[r#"
        model A {
          id String @id @default(auto()) @map("_id") @db.ObjectId
        }
    "#]];
    expected_doc.assert_eq(res.datamodel());
}
