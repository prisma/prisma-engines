use query_engine_tests::*;
use std::sync::Arc;

#[test_suite]
mod occ {
    pub fn occ_simple() -> String {
        include_str!("occ_simple.prisma").to_owned()
    }

    async fn create_one_seat(runner: Arc<Runner>) {
        runner
            .query(r#"mutation { createOneSeat(data: { movie: "zardoz", id: 1 }) { id } }"#)
            .await
            .unwrap()
            .assert_success();
    }

    async fn create_one_user(user_id: u64, runner: Arc<Runner>) {
        let query = format!(r#"mutation {{ createOneUser(data: {{ id: {user_id} }}) {{ id }} }}"#);
        runner.query(query).await.unwrap().assert_success();
    }

    async fn find_unclaimed_seat(runner: Arc<Runner>) -> (u64, u64) {
        let seat_query = "query { findFirstSeat(where: { movie: \"zardoz\", userId: null }) { id version } }";
        let seat_result = runner.query(seat_query).await.unwrap().to_json_value();
        let available_seat = &seat_result["data"]["findFirstSeat"];
        match available_seat {
            serde_json::Value::Null => (0, 0),
            other => (other["id"].as_u64().unwrap(), other["version"].as_u64().unwrap()),
        }
    }

    async fn book_unclaimed_seat(user_id: u64, seat_id: u64, runner: Arc<Runner>) -> (u64, u64) {
        let query = indoc::formatdoc!(
            r##"
                      mutation {{
                          updateManySeat(
                              data: {{ userId: {user_id}, version: {{ increment: 1 }} }},
                              where: {{ id: {seat_id}, version: 0 }}
                          )
                          {{ count }}
                      }}
                    "##
        );
        let response = runner.query(query).await.unwrap().to_json_value();
        let seat_count = response["data"]["updateManySeat"]["count"].as_u64().unwrap();
        (user_id, seat_count)
    }

    async fn book_seat_for_user(user_id: u64, runner: Arc<Runner>) -> (u64, u64) {
        let (seat_id, _version) = find_unclaimed_seat(runner.clone()).await;
        book_unclaimed_seat(user_id, seat_id, runner).await
    }

    async fn delete_seats(runner: Arc<Runner>) {
        let delete_seats = r#"
            mutation {
                deleteManySeat(where: {}) {
                count
                }
            }
        "#;
        runner.query(delete_seats).await.unwrap().assert_success();
    }

    async fn delete_users(runner: Arc<Runner>) {
        let delete_users = r#"
            mutation {
                deleteManyUser(where: {}) {
                count
                }
            }
        "#;
        runner.query(delete_users).await.unwrap().assert_success();
    }

    async fn run_occ_reproduce_test(runner: Arc<Runner>) {
        const USERS_COUNT: u64 = 5;

        create_one_seat(runner.clone()).await;

        for i in 0..=USERS_COUNT {
            create_one_user(i, runner.clone()).await;
        }

        let mut set = tokio::task::JoinSet::new();
        for user_id in 0..=USERS_COUNT {
            set.spawn(book_seat_for_user(user_id, runner.clone()));
        }

        let mut booked_user_id = 100;
        let mut total_booked = 0;
        while let Some(res) = set.join_next().await {
            let (user_id, count) = res.unwrap();

            if count > 0 {
                total_booked += count;
                booked_user_id = user_id;
            }
        }

        assert_eq!(total_booked, 1);

        let booked_seat = runner
            .query("query { findFirstSeat { id version userId } }")
            .await
            .unwrap()
            .to_json_value();

        let found_booked_user_id = booked_seat["data"]["findFirstSeat"]["userId"].as_u64().unwrap();

        assert_eq!(booked_user_id, found_booked_user_id);
    }

    // On PlanetScale, this fails with:
    // ```
    // assertion `left == right` failed
    // left: 6
    // right: 1
    // ```
    //
    // On D1, this fails with:
    // ```
    // assertion `left == right` failed
    // left: 3
    // right: 1
    // ```
    #[connector_test(
        schema(occ_simple),
        exclude(MongoDB, CockroachDb, Vitess("planetscale.js.wasm"), Sqlite("cfd1"))
    )]
    async fn occ_update_many_test(runner: Runner) -> TestResult<()> {
        let runner = Arc::new(runner);

        // This test can give false positives so we run it a few times
        // to make sure.
        for _ in 0..=5 {
            delete_seats(runner.clone()).await;
            delete_users(runner.clone()).await;
            run_occ_reproduce_test(runner.clone()).await;
        }

        Ok(())
    }

    #[connector_test(schema(occ_simple), exclude(CockroachDb, Vitess("planetscale.js.wasm")))]
    async fn occ_update_test(runner: Runner) -> TestResult<()> {
        let runner = Arc::new(runner);

        create_one_resource(runner.clone()).await;

        let mut set = tokio::task::JoinSet::new();

        set.spawn(update_one_resource(runner.clone()));
        set.spawn(update_one_resource(runner.clone()));
        set.spawn(update_one_resource(runner.clone()));
        set.spawn(update_one_resource(runner.clone()));

        while (set.join_next().await).is_some() {}

        let res = find_one_resource(runner).await;

        let expected = serde_json::json!({
            "data": {
            "findFirstResource": {
              "occStamp": 1,
              "id": 1
            }
          }
        });

        assert_eq!(res, expected);

        Ok(())
    }

    #[connector_test(schema(occ_simple), exclude(Vitess("planetscale.js.wasm")))]
    async fn occ_delete_test(runner: Runner) -> TestResult<()> {
        let runner = Arc::new(runner);

        create_one_resource(runner.clone()).await;

        let mut set = tokio::task::JoinSet::new();

        set.spawn(update_and_delete(runner.clone()));
        set.spawn(update_and_delete(runner.clone()));
        set.spawn(update_and_delete(runner.clone()));
        set.spawn(update_and_delete(runner.clone()));
        set.spawn(update_and_delete(runner.clone()));

        while (set.join_next().await).is_some() {}

        let res = find_one_resource(runner).await;

        let expected = serde_json::json!({
            "data": {
            "findFirstResource": {
              "occStamp": 1,
              "id": 1
            }
          }
        });

        assert_eq!(res, expected);

        Ok(())
    }

    #[connector_test(schema(occ_simple))]
    async fn occ_delete_many_test(runner: Runner) -> TestResult<()> {
        let runner = Arc::new(runner);

        create_one_resource(runner.clone()).await;

        let mut set = tokio::task::JoinSet::new();

        set.spawn(delete_many_resource(runner.clone()));
        set.spawn(delete_many_resource(runner.clone()));
        set.spawn(delete_many_resource(runner.clone()));
        set.spawn(delete_many_resource(runner.clone()));
        set.spawn(delete_many_resource(runner.clone()));

        let mut num_deleted: u64 = 0;
        while let Some(res) = set.join_next().await {
            if let Ok(row_count) = res
                && row_count > 0
            {
                num_deleted += 1;
            }
        }

        assert_eq!(num_deleted, 1);
        let res = find_one_resource(runner).await;

        let expected = serde_json::json!({
            "data": {
            "findFirstResource": serde_json::Value::Null
          }
        });
        assert_eq!(res, expected);

        Ok(())
    }

    // Because of the way upsert works this test is a little bit flaky. Ignoring until we fix upsert
    #[allow(dead_code)]
    #[ignore]
    async fn occ_upsert_test(runner: Runner) -> TestResult<()> {
        let runner = Arc::new(runner);

        let mut set = tokio::task::JoinSet::new();

        set.spawn(upsert_one_resource(runner.clone()));
        set.spawn(upsert_one_resource(runner.clone()));
        set.spawn(upsert_one_resource(runner.clone()));
        set.spawn(upsert_one_resource(runner.clone()));
        set.spawn(upsert_one_resource(runner.clone()));

        while (set.join_next().await).is_some() {}

        let res = find_one_resource(runner.clone()).await;

        // MongoDB is different here and seems to only do one create with all the upserts
        // where as all the sql databases will do one create and one upsert
        let expected = if matches!(runner.connector_version(), ConnectorVersion::MongoDb(_)) {
            serde_json::json!({
                "data": {
                "findFirstResource": {
                  "occStamp": 0,
                  "id": 1
                }
              }
            })
        } else {
            serde_json::json!({
                "data": {
                "findFirstResource": {
                  "occStamp": 1,
                  "id": 1
                }
              }
            })
        };
        assert_eq!(res, expected);

        Ok(())
    }

    async fn update_and_delete(runner: Arc<Runner>) {
        update_one_resource(runner.clone()).await;
        delete_one_resource(runner).await;
    }

    async fn create_one_resource(runner: Arc<Runner>) {
        let create_one_resource = r#"
        mutation {
            createOneResource(data: {id: 1}) {
              id
            }
          }"#;

        runner.query(create_one_resource).await.unwrap().to_json_value();
    }

    async fn update_one_resource(runner: Arc<Runner>) -> serde_json::Value {
        let update_one_resource = r#"
        mutation {
            updateOneResource(data: {occStamp: {increment: 1}}, where: {occStamp: 0}) {
              occStamp,
              id
            }
          }
        "#;

        runner.query(update_one_resource).await.unwrap().to_json_value()
    }

    #[allow(dead_code)]
    async fn upsert_one_resource(runner: Arc<Runner>) -> serde_json::Value {
        let upsert_one_resource = r#"
        mutation {
            upsertOneResource(where: {occStamp: 0}, 
             create: {
               occStamp: 0,
               id: 1
             },
             update: {
                 occStamp: {increment: 1}
               }) {
             id,
             occStamp
            }
           }
        "#;

        runner.query(upsert_one_resource).await.unwrap().to_json_value()
    }

    async fn delete_one_resource(runner: Arc<Runner>) -> serde_json::Value {
        let delete_one_resource = r#"
        mutation {
            deleteOneResource(where: {occStamp: 0}) {
              occStamp,
              id
            }
          }
        "#;

        runner.query(delete_one_resource).await.unwrap().to_json_value()
    }

    async fn delete_many_resource(runner: Arc<Runner>) -> u64 {
        let delete_many_resource = r#"
        mutation {
            deleteManyResource(where: {occStamp: 0}) {
              count
            }
          }
        "#;

        let res = runner.query(delete_many_resource).await.unwrap().to_json_value();

        res["data"]["deleteManyResource"]["count"].as_u64().unwrap()
    }

    async fn find_one_resource(runner: Arc<Runner>) -> serde_json::Value {
        let find_one_resource = r#"
        {
            findFirstResource(where: {}) {
                occStamp,
                id
            }
        }
        "#;

        runner.query(find_one_resource).await.unwrap().to_json_value()
    }
}
