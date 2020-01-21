use super::super::test_harness::*;

// Blocked on https://github.com/prisma/prisma-engine/pull/341
//
// #[test_each_connector]
// async fn index_on_compound_relation_fields_must_work(api: &TestApi) -> TestResult {
//     let dm = r#"
//         model User {
//             id String @id
//             email String
//             name String

//             @@unique([email, name])
//         }

//         model Post {
//             id String @id
//             author User @relation(references: [email, name])

//             @@index([author])
//         }
//     "#;

//     api.infer_apply(dm).send().await?;

//     api.assert_schema()
//         .await?
//         .assert_table("Post", |table| {
//             table
//                 .assert_has_column("username")?
//                 .assert_index_on_columns(&["a", "b"], |idx| Ok(idx))
//         })
//         .map(drop)
// }

#[test_each_connector]
async fn index_settings_must_be_migrated(api: &TestApi) -> TestResult {
    let dm = r#"
        model Test {
            id String @id
            name String
            followersCount Int

            @@index([name, followersCount], name: "nameAndFollowers")
        }
    "#;

    api.infer_apply(dm).send().await?;

    api.assert_schema().await?.assert_table("Test", |table| {
        table
            .assert_indexes_count(1)?
            .assert_index_on_columns(&["name", "followersCount"], |idx| idx.assert_is_not_unique())
    })?;

    let dm2 = r#"
        model Test {
            id String @id
            name String
            followersCount Int

            @@unique([name, followersCount], name: "nameAndFollowers")
        }
    "#;

    api.infer_apply(dm2).send().await?;

    api.assert_schema()
        .await?
        .assert_table("Test", |table| {
            table
                .assert_indexes_count(1)?
                .assert_index_on_columns(&["name", "followersCount"], |idx| idx.assert_is_unique())
        })
        .map(drop)
}
