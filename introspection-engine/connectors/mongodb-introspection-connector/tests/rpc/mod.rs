use crate::test_api::*;

#[test]
fn mongo_database_description() {
    let res = get_database_description(|db| async move {
        db.create_collection("A", None).await?;
        Ok(())
    });

    let expected = expect![[r#"
        {
          "collections": [
            {
              "name": "A"
            }
          ],
          "indexes": [],
          "collection_indexes": {}
        }"#]];

    expected.assert_eq(&res);
}

#[test]
fn mongo_database_version() {
    let res = get_database_version(|db| async move {
        db.create_collection("A", None).await?;
        Ok(())
    });

    assert!(res.contains("4.4.") || res.contains("5.0") || res.contains("4.2"))
}
