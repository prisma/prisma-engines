use crate::test_api::*;
use mongodb::bson::doc;
use serde_json::json;

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
          id  String @id @default(auto()) @map("_id") @db.ObjectId
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
          id String @id @default(auto()) @map("_id") @db.ObjectId

          @@map("?A")
        }

        model A_b_c {
          id String @id @default(auto()) @map("_id") @db.ObjectId

          @@map("A b c")
        }
    "#]];

    expected.assert_eq(res.datamodel());
}

#[test]
fn remapping_composite_fields_with_numbers() {
    let res = introspect(|db| async move {
        db.collection("Outer")
            .insert_one(
                doc! {
                    "inner": {
                        "1": 1,
                    },
                },
                None,
            )
            .await?;

        Ok(())
    });

    let expected = expect![[r#"
        type OuterInner {
          // This field was commented out because of an invalid name. Please provide a valid one that matches [a-zA-Z][a-zA-Z0-9_]*
          // 1 Int @map("1")
        }

        model Outer {
          id    String     @id @default(auto()) @map("_id") @db.ObjectId
          inner OuterInner
        }
    "#]];

    expected.assert_eq(res.datamodel());

    res.assert_warning("These enum values were commented out because their names are currently not supported by Prisma. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]* using the `@map` attribute.");

    res.assert_warning_affected(&json!([{
        "type": "OuterInner",
        "field": "1",
    }]));
}

#[test]
fn remapping_model_fields_with_numbers() {
    let res = introspect(|db| async move {
        db.collection("Outer")
            .insert_one(
                doc! {
                    "1": 1,
                },
                None,
            )
            .await?;

        Ok(())
    });

    let expected = expect![[r#"
        model Outer {
          id   String @id @default(auto()) @map("_id") @db.ObjectId
          // This field was commented out because of an invalid name. Please provide a valid one that matches [a-zA-Z][a-zA-Z0-9_]*
          // 1 Int    @map("1")
        }
    "#]];

    expected.assert_eq(res.datamodel());

    res.assert_warning("These enum values were commented out because their names are currently not supported by Prisma. Please provide valid ones that match [a-zA-Z][a-zA-Z0-9_]* using the `@map` attribute.");

    res.assert_warning_affected(&json!([{
        "model": "Outer",
        "field": "1",
    }]));
}

#[test]
fn remapping_model_fields_with_numbers_dirty() {
    let res = introspect(|db| async move {
        let docs = vec![doc! {"1": "Musti"}, doc! {"1": 1}];
        db.collection("Outer").insert_many(docs, None).await.unwrap();

        Ok(())
    });

    let expected = expect![[r#"
        model Outer {
          id   String @id @default(auto()) @map("_id") @db.ObjectId
          // Multiple data types found: String: 50%, Int32: 50% out of 2 sampled entries
          // This field was commented out because of an invalid name. Please provide a valid one that matches [a-zA-Z][a-zA-Z0-9_]*
          // 1 Int    @map("1")
        }
    "#]];

    expected.assert_eq(res.datamodel());
}
