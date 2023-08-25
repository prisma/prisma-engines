use crate::introspection::test_api::*;
use mongodb::{bson::doc, options::CreateCollectionOptions};

#[test]
fn collection_with_view() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;
        db.create_collection(
            "myView".to_string(),
            Some(
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
            ),
        )
        .await?;

        Ok(())
    });

    let expected_warning = expect![""];
    res.expect_warnings(&expected_warning);

    let expected_doc = expect![[r#"
        type System.viewsPipeline {
        lookup System.viewsPipeline$lookup @map("$lookup")
        }

        type System.viewsPipeline$lookup {
        as String
        foreignField String
        from String
        localField String
        }

        model A {
          id String @id @default(auto()) @map("_id") @db.ObjectId
        }

        model system_views {
          id     String @id @map("_id")
          pipeline System.viewsPipeline[]
          viewOn String

          @@map("system.views")
        }
    "#]];
    expected_doc.assert_eq(res.datamodel());
}
