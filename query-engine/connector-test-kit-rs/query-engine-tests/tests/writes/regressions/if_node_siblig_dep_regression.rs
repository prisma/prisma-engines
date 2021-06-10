use query_engine_tests::*;

// Related issue: https://github.com/prisma/prisma/issues/4230
#[test_suite]
mod if_node_sibling {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model Container {
              #id(id, Int, @id, @default(autoincrement()))
            
              Record Record[]
            }
            
            model RecordConfig {
              #id(id, Int, @id, @default(autoincrement()))
            
              Record Record[]
            }
            
            model RecordLocation {
              #id(id, Int, @id, @default(autoincrement()))
              location String @unique
            
              Record Record[]
            }
            
            model RecordType {
              #id(id, Int, @id, @default(autoincrement()))
              type   String   @unique
            
              Record Record[]
            }
            
            model Record {
              #id(id, Int, @id, @default(autoincrement()))
              location     RecordLocation @relation(fields: [locationId], references: [id])
              locationId   Int
              type         RecordType     @relation(fields: [recordTypeId], references: [id])
              recordTypeId Int
              config       RecordConfig?  @relation(fields: [configId], references: [id])
              configId     Int?
              container    Container      @relation(fields: [containerId], references: [id])
              containerId  Int
            }"#
        };

        schema.to_owned()
    }

    // "The if node sibling reordering" should "include all siblings that are not another if"
    #[connector_test(schema(schema))]
    async fn test(runner: &Runner) -> TestResult<()> {
        run_query!(runner, r#"mutation { createOneRecordConfig(data: {}) {id} }"#);
        run_query!(runner, r#"mutation { createOneContainer(data: {}) {id} }"#);

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneRecord(data:{
              container: { connect: { id: 1 }}
              config: { connect: { id: 1 }}
              location: {
                connectOrCreate: {
                  where: { location: "something" }
                  create: { location: "something" }
                }
              }
              type: {
                connectOrCreate: {
                  where: { type: "test" }
                  create: { type: "test" }
                }
              }
            }) {
              id
            }
          }"#),
          @r###"{"data":{"createOneRecord":{"id":1}}}"###
        );

        Ok(())
    }
}
