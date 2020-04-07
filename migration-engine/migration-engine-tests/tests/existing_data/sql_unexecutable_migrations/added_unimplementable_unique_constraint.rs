use migration_engine_tests::sql::*;

// TODO!
// #[test_each_connector(tags("sql"), log = "debug")]
// async fn adding_a_unique_constraint_when_existing_data_does_not_respect_it_is_unexecutable(
//     api: &TestApi,
// ) -> TestResult {
//     let dm1 = r#"
//         model Test {
//             id String @id
//             name String
//         }
//     "#;

//     api.infer_apply(&dm1).send().await?.assert_green()?;

//     {
//         api.insert("Test")
//             .value("id", "abc")
//             .value("name", "george")
//             .result_raw()
//             .await?;

//         api.insert("Test")
//             .value("id", "def")
//             .value("name", "george")
//             .result_raw()
//             .await?;
//     }

//     let dm2 = r#"
//         model Test {
//             id String @id
//             name String @unique
//         }
//     "#;

//     api.infer_apply(&dm2)
//         .force(Some(false))
//         .send()
//         .await?
//         .assert_unexecutable(&["yo".into()])?;

//     let rows = api.select("Test").column("id").column("name").send_debug().await?;
//     assert_eq!(
//         rows,
//         &[
//             &[r#"Text("abc")"#, r#"Text("george")"#],
//             &[r#"Text("def")"#, r#"Text("george")"#]
//         ]
//     );

//     Ok(())
// }

#[test_each_connector(tags("sql"))]
async fn adding_a_unique_constraint_when_existing_data_respects_it_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Test {
            id String @id
            name String
        }
    "#;

    api.infer_apply(&dm1).send().await?.assert_green()?;

    api.insert("Test")
        .value("id", "abc")
        .value("name", "george")
        .result_raw()
        .await?;

    api.insert("Test")
        .value("id", "def")
        .value("name", "georgina")
        .result_raw()
        .await?;

    let dm2 = r#"
        model Test {
            id String @id
            name String @unique
        }
    "#;

    api.infer_apply(&dm2).send().await?.assert_green()?;

    let rows = api.select("Test").column("id").column("name").send_debug().await?;
    assert_eq!(
        rows,
        &[
            &[r#"Text("abc")"#, r#"Text("george")"#],
            &[r#"Text("def")"#, r#"Text("georgina")"#]
        ]
    );

    Ok(())
}
