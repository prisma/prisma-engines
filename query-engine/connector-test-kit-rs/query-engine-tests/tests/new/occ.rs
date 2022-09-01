use query_engine_tests::*;
use std::sync::Arc;

#[test_suite]
mod occ {
    pub fn occ_simple() -> String {
        include_str!("occ_simple.prisma").to_owned()
    }

    #[connector_test(schema(occ_simple))]
    async fn occ_simple_test(runner: Runner) -> TestResult<()> {
        const USERS_COUNT: usize = 3;
        let runner = Arc::new(runner);

        // CREATE seat
        runner
            .query(r#"mutation { createOneSeat(data: { movie: "zardoz" }) { id } }"#)
            .await?
            .assert_success();

        // CREATE users
        for i in 1..=USERS_COUNT {
            let query = format!(r#"mutation {{ createOneUser(data: {{ id: {i} }}) {{ id }} }}"#);
            runner.query(query).await?.assert_success();
        }

        let (sender, mut receiver) = tokio::sync::mpsc::channel::<(usize, Option<u64>)>(USERS_COUNT);

        for i in 1..=USERS_COUNT {
            let sender = sender.clone();
            let runner = Arc::clone(&runner);
            tokio::spawn(async move {
                let seat_query = "query { findFirstSeat(where: { movie: \"zardoz\", userId: null }) { id version } }";
                let seat_result = runner.query(seat_query).await.unwrap().to_json_value();
                let available_seat = &seat_result["data"]["findFirstSeat"];
                let (available_seat_id, available_seat_version) = match available_seat {
                    serde_json::Value::Null => {
                        tracing::info!("no available seat for user {i}");
                        sender.send((i, None)).await.unwrap();
                        return Ok(());
                    }
                    other => (other["id"].as_u64().unwrap(), other["version"].as_u64().unwrap()),
                };

                let query = indoc::formatdoc!(
                    r##"
                      mutation {{
                          updateManySeat(
                              data: {{ userId: {i}, version: {{ increment: 1 }} }},
                              where: {{ id: {available_seat_id}, version: {available_seat_version} }}
                          )
                          {{ count }}
                      }}
                    "##
                );
                let response = dbg!(runner.query(query).await?.to_json_value());
                let seat_count = response["data"]["updateManySeat"]["count"].as_u64().unwrap();
                sender.send((i, Some(seat_count))).await.unwrap();
                TestResult::<()>::Ok(())
            });
        }

        let mut results = Vec::new();

        for _ in 0..USERS_COUNT {
            results.push(receiver.recv().await.unwrap());
        }

        let booked_seat = runner
            .query("query { findFirstSeat { id version userId } }")
            .await?
            .to_json_value();
        panic!("{:#?}", (results, booked_seat));

        // // READ
        // assert_query!(
        //     runner,
        //     "query { findFirstTestModel(where: { id: 1 }) { id }}",
        //     r#"{"data":{"findFirstTestModel":{"id":1}}}"#
        // );

        // assert_query!(
        //     runner,
        //     "query { findFirstTestModel2(where: { id: 1 }) { id, number }}",
        //     r#"{"data":{"findFirstTestModel2":{"id":1,"number":1}}}"#
        // );

        // // UPDATE
        // assert_query!(
        //     runner,
        //     r#"mutation { updateOneTestModel(where: { id: 1 }, data: { field: "two" }) { id } }"#,
        //     r#"{"data":{"updateOneTestModel":{"id":1}}}"#
        // );

        // assert_query!(
        //     runner,
        //     r#"mutation { updateOneTestModel2(where: { id: 1 }, data: { number: 2 }) { id } }"#,
        //     r#"{"data":{"updateOneTestModel2":{"id":1}}}"#
        // );

        // assert_query!(
        //     runner,
        //     "query { findFirstTestModel(where: { id: 1 }) { id, field }}",
        //     r#"{"data":{"findFirstTestModel":{"id":1,"field":"two"}}}"#
        // );

        // assert_query!(
        //     runner,
        //     "query { findFirstTestModel2(where: { id: 1 }) { id, number }}",
        //     r#"{"data":{"findFirstTestModel2":{"id":1,"number":2}}}"#
        // );

        // // DELETE

        // assert_query!(
        //     runner,
        //     "mutation { deleteOneTestModel(where: {id: 1}) { id } }",
        //     r#"{"data":{"deleteOneTestModel":{"id":1}}}"#
        // );

        // assert_query!(
        //     runner,
        //     "mutation { deleteOneTestModel2(where: {id: 1}) { id } }",
        //     r#"{"data":{"deleteOneTestModel2":{"id":1}}}"#
        // );

        // assert_query!(
        //     runner,
        //     "query { findFirstTestModel(where: { id: 1 }) { id, field }}",
        //     r#"{"data":{"findFirstTestModel":null}}"#
        // );

        // assert_query!(
        //     runner,
        //     "query { findFirstTestModel2(where: { id: 1 }) { id, number }}",
        //     r#"{"data":{"findFirstTestModel2":null}}"#
        // );

        Ok(())
    }
}
