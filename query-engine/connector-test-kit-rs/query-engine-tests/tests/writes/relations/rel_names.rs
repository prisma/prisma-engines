use query_engine_tests::*;

// We were creating a child_a record instead of child_b.
// https://github.com/prisma/prisma/issues/14696
//
// This was due to a relation resolution logic issue.
//
// exclude: mongodb has very specific constraint on id fields
#[test_suite(exclude(MongoDB))]
mod rel_names {
    fn schema() -> String {
        let schema = r#"
            model parent {
              id          String   @id
              child_a_id  String?
              child_b_id  String?
              child_a_rel child_a? @relation("child", fields: [child_a_id], references: [id])
              child_b_rel child_b? @relation("child", fields: [child_b_id], references: [id])
            }

            model child_a {
              id     String   @id
              name String?
              parent parent[] @relation("child")
            }

            model child_b {
              id     String   @id
              name String?
              parent parent[] @relation("child")
            }
        "#;

        schema.to_owned()
    }

    #[connector_test(schema(schema))]
    async fn relation_names_are_resolved_correctly_in_create(runner: Runner) -> TestResult<()> {
        let result = runner.query(r#"
            mutation { createOneparent(data: { id: "theparent", child_b_rel: { create: { id: "the_child_emphatically_b" } } }) { id } }
        "#).await?;
        result.assert_success();

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManychild_a { id } }"#),
          @r###"{"data":{"findManychild_a":[]}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManychild_b { id } }"#),
          @r###"{"data":{"findManychild_b":[{"id":"the_child_emphatically_b"}]}}"###
        );

        Ok(())
    }
}
