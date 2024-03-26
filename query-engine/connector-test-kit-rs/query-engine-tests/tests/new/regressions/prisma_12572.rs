use query_engine_tests::*;

// The fix for this issue caused problems with `create` operations. See the comment and tests in
// `prisma_15581.rs`.
#[test_suite(schema(schema))]
mod prisma_12572 {
    fn schema() -> String {
        r#"
            model Test1 {
                #id(id, String, @id)
                up1 DateTime @updatedAt
                cr1 DateTime @default(now())
                cr2 DateTime @default(now())
                up2 DateTime @updatedAt
                test2s Test2[]
            }

            model Test2 {
                #id(id, String, @id)
                test1Id String @unique
                test1 Test1 @relation(fields: [test1Id], references: [id])
                cr DateTime @default(now())
                up DateTime @updatedAt
            }
        "#
        .to_owned()
    }

    #[connector_test(exclude(Sqlite("cfd1")))]
    async fn all_generated_timestamps_are_the_same(runner: Runner) -> TestResult<()> {
        runner
            .query(r#"mutation { createOneTest1(data: {id:"one", test2s: { create: {id: "two"}}}) { id }}"#)
            .await?
            .assert_success();
        let testones = runner.query(r#"{ findManyTest1 { id up1 cr1 cr2 up2 } }"#).await?;
        let testtwos = runner.query(r#"{ findManyTest2 { id up cr } }"#).await?;
        testones.assert_success();
        testtwos.assert_success();

        let testones_json = testones.to_json_value();
        let testtwos_json = testtwos.to_json_value();
        let testone_obj = &testones_json["data"]["findManyTest1"][0];
        let testtwo_obj = &testtwos_json["data"]["findManyTest2"][0];

        let values = &[
            &testone_obj["up1"].as_str().unwrap(),
            &testone_obj["up2"].as_str().unwrap(),
            &testone_obj["cr1"].as_str().unwrap(),
            &testone_obj["cr2"].as_str().unwrap(),
            &testtwo_obj["up"].as_str().unwrap(),
            &testtwo_obj["cr"].as_str().unwrap(),
        ];

        // assert that all the datetimes are the same
        for datetimes in values.windows(2) {
            assert_eq!(datetimes[0], datetimes[1]);
        }

        Ok(())
    }
}
