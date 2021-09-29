use crate::test_api::*;

#[test]
fn remapping_fields_with_invalid_characters() {
    let res = introspect(|db| async move {
        db.create_collection("A", None).await?;

        db.collection("A")
            .insert_one(
                doc! {
                    "_a": 1,
                    "*b": 2,
                    "?c": 3,
                    "(d": 4,
                    ")e": 5,
                    "/f": 6,
                    "g a": 7,
                    "h-a": 8,
                    "h1": 9,
                },
                None,
            )
            .await?;

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id  String @id @default(dbgenerated()) @map("_id") @db.ObjectId
          d   Int    @map("(d")
          e   Int    @map(")e")
          b   Int    @map("*b")
          f   Int    @map("/f")
          c   Int    @map("?c")
          a   Int    @map("_a")
          g_a Int    @map("g a")
          h_a Int    @map("h-a")
          h1  Int
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn remapping_models_with_invalid_characters() {
    let res = introspect(|db| async move {
        db.create_collection("?A", None).await?;
        db.create_collection("A b c", None).await?;

        Ok(())
    });

    let expected = expect![[r#"
        model A {
          id String @id @default(dbgenerated()) @map("_id") @db.ObjectId

          @@map("?A")
        }

        model A_b_c {
          id String @id @default(dbgenerated()) @map("_id") @db.ObjectId

          @@map("A b c")
        }
    "#]];

    expected.assert_eq(res.datamodel());
}
