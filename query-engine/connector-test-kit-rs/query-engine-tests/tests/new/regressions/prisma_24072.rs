use indoc::indoc;
use query_engine_tests::*;

// Skip databases that don't support `onDelete: SetDefault`
#[test_suite(
    schema(schema),
    exclude(MongoDb, MySql(5.6), MySql(5.7), Vitess("planetscale.js.wasm"))
)]
mod prisma_24072 {
    fn schema() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                child Child?
            }

            model Child {
                #id(id, Int, @id)
                parent_id Int?    @default(2) @unique
                parent    Parent? @relation(fields: [parent_id], references: [id], onDelete: NoAction)
            }"#
        };

        schema.to_owned()
    }

    // Deleting the parent without cascading to the child should fail with an explicitly named constraint violation,
    // without any "(not available)" names.
    #[connector_test]
    async fn test_24072(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        assert_connector_error!(
          &runner,
          "mutation { deleteOneParent(where: { id: 1 }) { id }}",
          2003,
          CockroachDb(_) | Postgres(_) | SqlServer(_) | Vitess(_) => "Foreign key constraint violated on the constraint: `Child_parent_id_fkey`",
          MySql(_) => "Foreign key constraint violated on the fields: (`parent_id`)",
          Sqlite(_) => "Foreign key constraint violated on the foreign key",
          _ => "Foreign key constraint violated"
        );

        Ok(())
    }
}
