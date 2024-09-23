use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema))]
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
                parent    Parent? @relation(fields: [parent_id], references: [id], onDelete: SetDefault)
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
          CockroachDb(_) | Postgres(_) | SqlServer(_) | Vitess(_) => "Foreign key constraint violated: `Child_parent_id_fkey (index)`",
          MySql(_) => "Foreign key constraint violated: `parent_id`",
          Sqlite(_) => "Foreign key constraint violated: `foreign key`",
          _ => "Foreign key constraint violated"
        );

        Ok(())
    }
}
