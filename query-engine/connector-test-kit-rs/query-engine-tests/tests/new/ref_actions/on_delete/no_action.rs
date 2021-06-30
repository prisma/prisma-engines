use indoc::indoc;
use query_engine_tests::*;

#[test_suite(suite = "noaction_onD_1to1_req", schema(required))]
mod one2one_req {
    fn required() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                child Child?
            }

            model Child {
                #id(id, Int, @id)
                parent_id Int
                parent    Parent @relation(fields: [parent_id], references: [id], onDelete: NoAction)
            }"#
        };

        schema.to_owned()
    }

    /// Deleting the parent must fail if a child is connected.
    #[connector_test(exclude(MongoDb))]
    async fn delete_parent_failure(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { createOneParent(data: { id: 1, child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        assert_error!(
            runner,
            "mutation { deleteOneParent(where: { id: 1 }) { id }}",
            2003,
            "Foreign key constraint failed on the field"
        );

        assert_error!(
            runner,
            "mutation { deleteManyParent(where: { id: 1 }) { count }}",
            2003,
            "Foreign key constraint failed on the field"
        );

        Ok(())
    }

    /// Deleting the parent leaves the data in a integrity-violating state.
    #[connector_test(only(MongoDb))]
    async fn delete_parent_violation(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { createOneParent(data: { id: 1, child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteOneParent(where: { id: 1 }) { id }}"#),
          @r###"{"data":{"deleteOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyChild { parent_id }}"#),
          @r###"{"data":{"findManyChild":[{"parent_id":1}]}}"###
        );

        Ok(())
    }
}

#[test_suite(suite = "noaction_onD_1to1_opt", schema(optional), exclude(MongoDb))]
mod one2one_opt {
    fn optional() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                child Child?
            }

            model Child {
                #id(id, Int, @id)
                parent_id Int?
                parent    Parent? @relation(fields: [parent_id], references: [id], onDelete: NoAction)
            }"#
        };

        schema.to_owned()
    }

    /// Deleting the parent must fail if a child is connected.
    #[connector_test]
    async fn delete_parent_failure(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { createOneParent(data: { id: 1, child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        assert_error!(
            runner,
            "mutation { deleteOneParent(where: { id: 1 }) { id }}",
            2003,
            "Foreign key constraint failed on the field"
        );

        assert_error!(
            runner,
            "mutation { deleteManyParent(where: { id: 1 }) { count }}",
            2003,
            "Foreign key constraint failed on the field"
        );

        Ok(())
    }

    /// Deleting the parent succeeds if no child is connected.
    #[connector_test]
    async fn delete_parent(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { createOneParent(data: { id: 1 }) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { createOneParent(data: { id: 2 }) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":2}}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, "mutation { deleteOneParent(where: { id: 1 }) { id }}"),
            @r###"{"data":{"deleteOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, "mutation { deleteManyParent(where: { id: 2 }) { count }}"),
            @r###"{"data":{"deleteManyParent":{"count":1}}}"###
        );

        Ok(())
    }

    /// Deleting the parent leaves the data in a integrity-violating state.
    #[connector_test(only(MongoDb))]
    async fn delete_parent_violation(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { createOneParent(data: { id: 1, child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteOneParent(where: { id: 1 }) { id }}"#),
          @r###"{"data":{"deleteOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyChild { parent_id }}"#),
          @r###"{"data":{"findManyChild":[{"parent_id":1}]}}"###
        );

        Ok(())
    }
}

#[test_suite(suite = "noaction_onD_1toM_req", schema(required), exclude(MongoDb))]
mod one2many_req {
    fn required() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                children Child[]
            }

            model Child {
                #id(id, Int, @id)
                parent_id Int
                parent    Parent @relation(fields: [parent_id], references: [id], onDelete: NoAction)
            }"#
        };

        schema.to_owned()
    }

    /// Deleting the parent must fail if a child is connected.
    #[connector_test]
    async fn delete_parent_failure(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { createOneParent(data: { id: 1, children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        assert_error!(
            runner,
            "mutation { deleteOneParent(where: { id: 1 }) { id }}",
            2003,
            "Foreign key constraint failed on the field"
        );

        assert_error!(
            runner,
            "mutation { deleteManyParent(where: { id: 1 }) { count }}",
            2003,
            "Foreign key constraint failed on the field"
        );

        Ok(())
    }

    /// Deleting the parent succeeds if no child is connected.
    #[connector_test]
    async fn delete_parent(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { createOneParent(data: { id: 1 }) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { createOneParent(data: { id: 2 }) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":2}}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, "mutation { deleteOneParent(where: { id: 1 }) { id }}"),
            @r###"{"data":{"deleteOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, "mutation { deleteManyParent(where: { id: 2 }) { count }}"),
            @r###"{"data":{"deleteManyParent":{"count":1}}}"###
        );

        Ok(())
    }

    /// Deleting the parent leaves the data in a integrity-violating state.
    #[connector_test(only(MongoDb))]
    async fn delete_parent_violation(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { createOneParent(data: { id: 1, children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteOneParent(where: { id: 1 }) { id }}"#),
          @r###"{"data":{"deleteOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyChild { parent_id }}"#),
          @r###"{"data":{"findManyChild":[{"parent_id":1}]}}"###
        );

        Ok(())
    }
}

#[test_suite(suite = "noaction_onD_1toM_opt", schema(optional), exclude(MongoDb))]
mod one2many_opt {
    fn optional() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                children Child[]
            }

            model Child {
                #id(id, Int, @id)
                parent_id Int?
                parent    Parent? @relation(fields: [parent_id], references: [id], onDelete: NoAction)
            }"#
        };

        schema.to_owned()
    }

    /// Deleting the parent must fail if a child is connected.
    #[connector_test]
    async fn delete_parent_failure(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { createOneParent(data: { id: 1, children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        assert_error!(
            runner,
            "mutation { deleteOneParent(where: { id: 1 }) { id }}",
            2003,
            "Foreign key constraint failed on the field"
        );

        assert_error!(
            runner,
            "mutation { deleteManyParent(where: { id: 1 }) { count }}",
            2003,
            "Foreign key constraint failed on the field"
        );

        Ok(())
    }

    /// Deleting the parent succeeds if no child is connected.
    #[connector_test]
    async fn delete_parent(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { createOneParent(data: { id: 1 }) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { createOneParent(data: { id: 2 }) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":2}}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, "mutation { deleteOneParent(where: { id: 1 }) { id }}"),
            @r###"{"data":{"deleteOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, "mutation { deleteManyParent(where: { id: 2 }) { count }}"),
            @r###"{"data":{"deleteManyParent":{"count":1}}}"###
        );

        Ok(())
    }

    /// Deleting the parent leaves the data in a integrity-violating state.
    #[connector_test(only(MongoDb))]
    async fn delete_parent_violation(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { createOneParent(data: { id: 1, children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { deleteOneParent(where: { id: 1 }) { id }}"#),
          @r###"{"data":{"deleteOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyChild { parent_id }}"#),
          @r###"{"data":{"findManyChild":[{"parent_id":1}]}}"###
        );

        Ok(())
    }
}
