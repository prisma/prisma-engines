use query_engine_tests::*;

#[test_suite]
mod pagination_regr {
    use indoc::indoc;
    use query_engine_tests::{assert_query_many, run_query};

    fn schema_2855() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, Int, @id)
              bs ModelB[]
            }
            
            model ModelB {
              #id(id, String, @id)
              createdAt DateTime @default(now())
              a_id Int
              a ModelA @relation(fields: [a_id], references: [id])
            }"#
        };

        schema.to_owned()
    }

    // "[prisma/2855] Duplicate ordering keys on non-sequential IDs" should "still allow paging through records predictably"
    #[connector_test(schema(schema_2855))]
    async fn prisma_2855(runner: &Runner) -> TestResult<()> {
        create_test_data_2855(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyModelB(take: 5, orderBy: [{ createdAt: desc}, { id: asc }]) {
              id
              createdAt
            }
          }"#),
          @r###"{"data":{"findManyModelB":[{"id":"7e00aa78-5951-4c05-8e42-4edb0927e964","createdAt":"2020-06-25T20:05:38.000Z"},{"id":"84c01d52-838d-4cdd-9035-c09cf54a06a0","createdAt":"2020-06-25T19:44:50.000Z"},{"id":"3e7d6b95-c62d-4e66-bb8c-66a317386e40","createdAt":"2020-06-19T21:32:11.000Z"},{"id":"99f1734d-6ad1-4cf0-b851-2ed551cbabc6","createdAt":"2020-06-19T21:32:02.000Z"},{"id":"9505b8a9-45a1-4aae-a284-5bacfe9f835c","createdAt":"2020-06-19T21:31:51.000Z"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyModelB(cursor: { id: "9505b8a9-45a1-4aae-a284-5bacfe9f835c" }, skip: 1, take: 5, orderBy: [{ createdAt: desc}, { id: asc }] ) {
              id
              createdAt
            }
          }"#),
          @r###"{"data":{"findManyModelB":[{"id":"ea732052-aac6-429b-84ea-976ca1f645d0","createdAt":"2020-06-11T22:34:15.000Z"},{"id":"13394728-24a6-4a37-aa6e-369e7f70c10b","createdAt":"2020-06-10T21:52:26.000Z"},{"id":"16fa1ce3-5243-4a30-970e-8ec98d077810","createdAt":"2020-06-10T21:52:26.000Z"},{"id":"36e88f2e-9f4c-4e26-9add-fbf76e404959","createdAt":"2020-06-10T21:52:26.000Z"},{"id":"3c0f269f-0796-427e-af67-8c1a99f3524d","createdAt":"2020-06-10T21:52:26.000Z"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyModelB(cursor: { id: "3c0f269f-0796-427e-af67-8c1a99f3524d" }, skip: 1, take: 5, orderBy: [{ createdAt: desc}, { id: asc }] ) {
              id
              createdAt
            }
          }"#),
          @r###"{"data":{"findManyModelB":[{"id":"517e8f7f-980a-44bf-8500-4e279a120b72","createdAt":"2020-06-10T21:52:26.000Z"},{"id":"620d09a6-f5bd-48b5-bbe6-d55fcf341392","createdAt":"2020-06-10T21:52:26.000Z"},{"id":"755f5bba-25e3-4510-a991-e0cfe02d864d","createdAt":"2020-06-10T21:52:26.000Z"},{"id":"8a49e477-1f12-4a81-953f-c7b0ca5696dc","createdAt":"2020-06-10T21:52:26.000Z"},{"id":"8c7a3864-285c-4f06-9c9a-273e19e19a05","createdAt":"2020-06-10T21:52:26.000Z"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyModelB(cursor: { id: "8c7a3864-285c-4f06-9c9a-273e19e19a05" }, skip: 1, take: 5, orderBy: [{ createdAt: desc}, { id: asc }] ) {
              id
              createdAt
            }
          }"#),
          @r###"{"data":{"findManyModelB":[{"id":"bae99648-bdad-440f-953b-ddab33c6ea0b","createdAt":"2020-06-10T21:52:26.000Z"},{"id":"eb8c5a20-ae61-402b-830f-f9518957f195","createdAt":"2020-06-10T21:52:26.000Z"},{"id":"79066f5a-3640-42e9-be04-2a702924f4c6","createdAt":"2020-06-04T16:00:21.000Z"},{"id":"a4b0472a-52fc-4b2d-8c44-4c401c18f469","createdAt":"2020-06-03T21:13:57.000Z"},{"id":"fc34b132-e376-406e-ab89-10ee35b4d58d","createdAt":"2020-05-12T12:30:12.000Z"}]}}"###
        );

        Ok(())
    }

    // "[prisma/3505][Case 1] Paging and ordering with potential null values ON a null row" should "still allow paging through records predictably"
    #[connector_test(schema(generic))]
    async fn prisma_3505_case_1(runner: &Runner) -> TestResult<()> {
        // 5 records with ids 1 to 5
        // Contain some nulls for `field`.
        create_test_data_3505_1(runner).await?;

        // Selects the 2 records after ID 2.
        // There are 2 options, depending on how the underlying db orders NULLS (first or last, * ids have nulls in `field`):
        // Nulls last:  5, 3, 1*, 2*, 4* => take only 4
        // Nulls first: 1*, 2*, 4*, 5, 3 => take 4, 5
        assert_query_many!(
            runner,
            r#"{
            findManyTestModel(
              cursor: { id: 2 },
              take: 2,
              skip: 1,
              orderBy: [{ field: desc }, { id: asc }]
            ) { id }
          }"#,
            vec![
                r#"{"data":{"findManyTestModel":[{"id":4}]}}"#,
                r#"{"data":{"findManyTestModel":[{"id":4},{"id":5}]}}"#
            ]
        );

        Ok(())
    }

    // "[prisma/3505][Case 2] Paging and ordering with potential null values NOT ON a null row" should "still allow paging through records predictably"
    // "Not on null row" means that the cursor row does not contain a null value for the ordering field, in this case row 2.
    // However, other rows might still have nulls, those must be taken into consideration.
    #[connector_test(schema(generic))]
    async fn prisma_3505_case_2(runner: &Runner) -> TestResult<()> {
        // 5 records with ids 1 to 5
        // Contain some nulls for `field`.
        create_test_data_3505_2(runner).await?;

        assert_query_many!(
            runner,
            r#"{
            findManyTestModel(
              cursor: { id: 5 },
              take: 2,
              skip: 1,
              orderBy: [{ field: desc }, { id: asc }]
            ) { id }
          }"#,
            vec![
                r#"{"data":{"findManyTestModel":[{"id":2}]}}"#,
                r#"{"data":{"findManyTestModel":[{"id":2},{"id":1}]}}"#
            ]
        );

        Ok(())
    }

    async fn create_test_data_3505_1(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1 }"#).await?;
        create_row(runner, r#"{ id: 2 }"#).await?;
        create_row(runner, r#"{ id: 3, field: "Test"}"#).await?;
        create_row(runner, r#"{ id: 4 }"#).await?;
        create_row(runner, r#"{ id: 5, field: "Test2"}"#).await?;

        Ok(())
    }

    async fn create_test_data_3505_2(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1 }"#).await?;
        create_row(runner, r#"{ id: 2, field: "Test"}"#).await?;
        create_row(runner, r#"{ id: 3 }"#).await?;
        create_row(runner, r#"{ id: 4 }"#).await?;
        create_row(runner, r#"{ id: 5, field: "Test2"}"#).await?;

        Ok(())
    }

    async fn create_test_data_2855(runner: &Runner) -> TestResult<()> {
        runner
            .query(
                r#"
            mutation {
              createOneModelA(
                data: {
                  id: 1
                  bs: {
                    create: [
                      {
                        id: "7e00aa78-5951-4c05-8e42-4edb0927e964"
                        createdAt: "2020-06-25T20:05:38.000Z"
                      }
                      {
                        id: "84c01d52-838d-4cdd-9035-c09cf54a06a0"
                        createdAt: "2020-06-25T19:44:50.000Z"
                      }
                      {
                        id: "3e7d6b95-c62d-4e66-bb8c-66a317386e40"
                        createdAt: "2020-06-19T21:32:11.000Z"
                      }
                      {
                        id: "99f1734d-6ad1-4cf0-b851-2ed551cbabc6"
                        createdAt: "2020-06-19T21:32:02.000Z"
                      }
                      {
                        id: "9505b8a9-45a1-4aae-a284-5bacfe9f835c"
                        createdAt: "2020-06-19T21:31:51.000Z"
                      }
                      {
                        id: "ea732052-aac6-429b-84ea-976ca1f645d0"
                        createdAt: "2020-06-11T22:34:15.000Z"
                      }
                      {
                        id: "13394728-24a6-4a37-aa6e-369e7f70c10b"
                        createdAt: "2020-06-10T21:52:26.000Z"
                      }
                      {
                        id: "16fa1ce3-5243-4a30-970e-8ec98d077810"
                        createdAt: "2020-06-10T21:52:26.000Z"
                      }
                      {
                        id: "36e88f2e-9f4c-4e26-9add-fbf76e404959"
                        createdAt: "2020-06-10T21:52:26.000Z"
                      }
                      {
                        id: "3c0f269f-0796-427e-af67-8c1a99f3524d"
                        createdAt: "2020-06-10T21:52:26.000Z"
                      }
                      {
                        id: "517e8f7f-980a-44bf-8500-4e279a120b72"
                        createdAt: "2020-06-10T21:52:26.000Z"
                      }
                      {
                        id: "620d09a6-f5bd-48b5-bbe6-d55fcf341392"
                        createdAt: "2020-06-10T21:52:26.000Z"
                      }
                      {
                        id: "755f5bba-25e3-4510-a991-e0cfe02d864d"
                        createdAt: "2020-06-10T21:52:26.000Z"
                      }
                      {
                        id: "8a49e477-1f12-4a81-953f-c7b0ca5696dc"
                        createdAt: "2020-06-10T21:52:26.000Z"
                      }
                      {
                        id: "8c7a3864-285c-4f06-9c9a-273e19e19a05"
                        createdAt: "2020-06-10T21:52:26.000Z"
                      }
                      {
                        id: "bae99648-bdad-440f-953b-ddab33c6ea0b"
                        createdAt: "2020-06-10T21:52:26.000Z"
                      }
                      {
                        id: "eb8c5a20-ae61-402b-830f-f9518957f195"
                        createdAt: "2020-06-10T21:52:26.000Z"
                      }
                      {
                        id: "79066f5a-3640-42e9-be04-2a702924f4c6"
                        createdAt: "2020-06-04T16:00:21.000Z"
                      }
                      {
                        id: "a4b0472a-52fc-4b2d-8c44-4c401c18f469"
                        createdAt: "2020-06-03T21:13:57.000Z"
                      }
                      {
                        id: "fc34b132-e376-406e-ab89-10ee35b4d58d"
                        createdAt: "2020-05-12T12:30:12.000Z"
                      }
                    ]
                  }
                }
              ) {
                id
              }
            }
            "#,
            )
            .await?
            .assert_success();
        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTestModel(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
