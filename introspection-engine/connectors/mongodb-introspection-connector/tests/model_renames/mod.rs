use crate::test_api::*;
use mongodb::bson::doc;

#[test]
fn a_model_with_reserved_name() {
    let res = introspect(|db| async move {
        db.create_collection("PrismaClient", None).await.unwrap();
        db.collection("PrismaClient")
            .insert_one(doc! {"data": 1}, None)
            .await
            .unwrap();

        Ok(())
    });

    let expected = expect![[r#"
        /// This model has been renamed to 'RenamedPrismaClient' during introspection, because the original name 'PrismaClient' is reserved.
        model RenamedPrismaClient {
          id   String @id @default(dbgenerated()) @map("_id") @db.ObjectId
          data Int

          @@map("PrismaClient")
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn reserved_names_case_sensitivity() {
    let res = introspect(|db| async move {
        db.create_collection("prismalclient", None).await.unwrap();
        db.collection("prismalclient")
            .insert_one(doc! {"data": 1}, None)
            .await
            .unwrap();

        Ok(())
    });

    let expected = expect![[r#"
        model prismalclient {
          id   String @id @default(dbgenerated()) @map("_id") @db.ObjectId
          data Int
        }
    "#]];

    expected.assert_eq(res.datamodel());
}
