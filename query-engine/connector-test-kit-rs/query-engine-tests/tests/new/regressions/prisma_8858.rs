use query_engine_tests::*;

#[test_suite(schema(schema), capabilities(CompoundIds))]
mod prisma_8858 {
    fn schema() -> String {
        r#"
        model Object {
          clientId  Int
          uuid      String
          otherUuid String?
        
          otherObject        Object? @relation("objectToobject", fields: [clientId, otherUuid], references: [clientId, uuid], onDelete: NoAction, onUpdate: NoAction)
          otherObjectReverse Object? @relation("objectToobject", onDelete: NoAction, onUpdate: NoAction)
        
          @@id([clientId, uuid])
          @@unique([clientId, otherUuid])
        }
        
        "#
        .to_owned()
    }

    #[connector_test]
    async fn regression_8858(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation { createOneObject(data: { clientId: 1, uuid: "1" }) { clientId } }"#
        );
        run_query!(
            &runner,
            r#"mutation { createOneObject(data: { clientId: 1, uuid: "2", otherUuid: "1" }) { clientId } }"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneObject(
          where: {
            clientId_uuid: {
              clientId: 1,
              uuid: "1",
            },
          },
          data: {
            otherUuid: "2",
          },
        ) { clientId uuid otherUuid } }"#),
          @r###"{"data":{"updateOneObject":{"clientId":1,"uuid":"1","otherUuid":"2"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneObject(
          where: {
            clientId_uuid: {
              clientId: 1,
              uuid: "1",
            },
          },
          data: {
            otherObject: {
              connect: {
                clientId_uuid: {
                  clientId: 1,
                  uuid: "2",
                },
              },
            },
          }
        ) { clientId uuid otherObject { clientId uuid } } }"#),
          @r###"{"data":{"updateOneObject":{"clientId":1,"uuid":"1","otherObject":{"clientId":1,"uuid":"2"}}}}"###
        );

        Ok(())
    }
}
