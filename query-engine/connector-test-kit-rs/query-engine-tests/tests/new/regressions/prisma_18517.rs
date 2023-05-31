use query_engine_tests::*;

#[test_suite(schema(schema), only(Postgres))]
mod prisma_18517 {
    fn schema() -> String {
        r#"
model Container {
  id                    String                 @id @default(uuid()) @test.Char(20)
  createdAt             DateTime               @default(now())
  label                 String?
  parent                Container?             @relation("ContainerToContainer", fields: [parentId], references: [id], onDelete: Cascade)
  parentId              String?                @test.Char(20)
  path                  Unsupported("money")?  @unique
  childContainers       Container[]            @relation("ContainerToContainer")
}
        "#.to_owned()
    }

    #[connector_test]
    async fn regression(runner: Runner) -> TestResult<()> {
        run_query! {
            &runner,
            r#"query { findManyContainer(where: { label: "root", parentId: null }) { id }}"#
        };
        Ok(())
    }
}
