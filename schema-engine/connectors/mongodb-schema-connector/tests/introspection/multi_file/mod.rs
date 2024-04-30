// TODO: Implement MongoDB reintrospection test case
// use crate::introspection::test_api::*;
// use mongodb::{bson::doc, options::CreateCollectionOptions};

// reintrospect_new_model_single_file
// reintrospect_new_model_multi_file
// reintrospect_removed_model_single_file
// reintrospect_removed_model_multi_file
// reintrospect_new_enum_single_file
// reintrospect_removed_enum_single_file
// reintrospect_new_enum_multi_file
// reintrospect_removed_enum_multi_file
// introspect_multi_view_preview_feature_is_required
// reintrospect_new_view_single_file
// reintrospect_removed_view_single_file
// reintrospect_new_view_multi_file
// reintrospect_removed_view_multi_file
// reintrospect_keep_configuration_in_same_file
// reintrospect_keep_configuration_when_spread_across_files
// reintrospect_keep_configuration_when_no_models
// reintrospect_empty_multi_file

// #[test]
// fn multiple_collections_with_data() {
//     let res = introspect(|db| async move {
//         db.create_collection("A", None).await?;
//         let collection = db.collection("A");
//         let docs = vec![doc! {"first": "Musti"}];

//         collection.insert_many(docs, None).await.unwrap();

//         db.create_collection("B", None).await?;
//         let collection = db.collection("B");
//         let docs = vec![doc! {"second": "Naukio"}];

//         collection.insert_many(docs, None).await.unwrap();

//         Ok(())
//     });

//     let expected = expect![[r#"
//         model A {
//           id    String @id @default(auto()) @map("_id") @db.ObjectId
//           first String
//         }

//         model B {
//           id     String @id @default(auto()) @map("_id") @db.ObjectId
//           second String
//         }
//     "#]];

//     expected.assert_eq(res.datamodel());
// }
