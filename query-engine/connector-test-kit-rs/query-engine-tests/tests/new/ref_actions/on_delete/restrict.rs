//! SQL Server doesn't support Restrict.

use indoc::indoc;
use query_engine_tests::*;

#[test_suite(
    suite = "restrict_onD_1to1_req",
    schema(required),
    exclude(SqlServer, Sqlite("cfd1")),
    relation_mode = "prisma"
)]
mod one2one_req {
    fn required() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                child Child?
            }

            model Child {
                #id(id, Int, @id)
                parent_id Int @unique
                parent    Parent @relation(fields: [parent_id], references: [id], onDelete: Restrict)
            }"#
        };

        schema.to_owned()
    }

    /// Deleting the parent must fail if a child is connected.
    #[connector_test]
    async fn delete_parent_failure(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        assert_error!(
            runner,
            "mutation { deleteOneParent(where: { id: 1 }) { id }}",
            2014,
            "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
        );

        Ok(())
    }
}

#[test_suite(
    suite = "restrict_onD_1to1_opt",
    schema(optional),
    exclude(SqlServer, Sqlite("cfd1")),
    relation_mode = "prisma"
)]
mod one2one_opt {
    fn optional() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                child Child?
            }

            model Child {
                #id(id, Int, @id)
                parent_id Int? @unique
                parent    Parent? @relation(fields: [parent_id], references: [id], onDelete: Restrict)
            }"#
        };

        schema.to_owned()
    }

    /// Deleting the parent must fail if a child is connected.
    #[connector_test]
    async fn delete_parent_failure(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        assert_error!(
            runner,
            "mutation { deleteOneParent(where: { id: 1 }) { id }}",
            2014,
            "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
        );

        Ok(())
    }

    /// Deleting the parent succeeds if no child is connected.
    #[connector_test]
    async fn delete_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1 }) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 2 }) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":2}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "mutation { deleteOneParent(where: { id: 1 }) { id }}"),
            @r###"{"data":{"deleteOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "mutation { deleteManyParent(where: { id: 2 }) { count }}"),
            @r###"{"data":{"deleteManyParent":{"count":1}}}"###
        );

        Ok(())
    }

    fn diff_id_name() -> String {
        let schema = indoc! {
            r#"model Parent {
            #id(id, Int, @id)
            uniq    Int? @unique
            child   Child?
          }
          
          model Child {
            #id(childId, Int, @id)
            childUniq       Int? @unique
            parent           Parent? @relation(fields: [childUniq], references: [uniq], onDelete: Restrict)
          }"#
        };

        schema.to_owned()
    }

    /// Deleting the parent succeeds if no child is connected.
    /// Checks that it works even with different parent/child primary identifier names.
    #[connector_test(schema(diff_id_name))]
    async fn delete_parent_diff_id_name(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation { createOneParent(data: { id: 1, uniq: 1 }) { id } }"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { deleteOneParent(where: { id: 1 }) { id } }"#),
          @r###"{"data":{"deleteOneParent":{"id":1}}}"###
        );

        Ok(())
    }
}

#[test_suite(
    suite = "restrict_onD_1toM_req",
    schema(required),
    exclude(SqlServer, Sqlite("cfd1")),
    relation_mode = "prisma"
)]
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
                parent    Parent @relation(fields: [parent_id], references: [id], onDelete: Restrict)
            }"#
        };

        schema.to_owned()
    }

    /// Deleting the parent must fail if a child is connected.
    #[connector_test]
    async fn delete_parent_failure(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        assert_error!(
            runner,
            "mutation { deleteOneParent(where: { id: 1 }) { id }}",
            2014,
            "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
        );

        Ok(())
    }

    /// Deleting the parent succeeds if no child is connected.
    #[connector_test]
    async fn delete_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1 }) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 2 }) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":2}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "mutation { deleteOneParent(where: { id: 1 }) { id }}"),
            @r###"{"data":{"deleteOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "mutation { deleteManyParent(where: { id: 2 }) { count }}"),
            @r###"{"data":{"deleteManyParent":{"count":1}}}"###
        );

        Ok(())
    }
}

#[test_suite(
    suite = "restrict_onD_1toM_opt",
    schema(optional),
    exclude(SqlServer, Sqlite("cfd1")),
    relation_mode = "prisma"
)]
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
                parent    Parent? @relation(fields: [parent_id], references: [id], onDelete: Restrict)
            }"#
        };

        schema.to_owned()
    }

    /// Deleting the parent must fail if a child is connected.
    #[connector_test]
    async fn delete_parent_failure(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        assert_error!(
            runner,
            "mutation { deleteOneParent(where: { id: 1 }) { id }}",
            2014,
            "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
        );

        Ok(())
    }

    /// Deleting the parent succeeds if no child is connected.
    #[connector_test]
    async fn delete_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1 }) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 2 }) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":2}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "mutation { deleteOneParent(where: { id: 1 }) { id }}"),
            @r###"{"data":{"deleteOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "mutation { deleteManyParent(where: { id: 2 }) { count }}"),
            @r###"{"data":{"deleteManyParent":{"count":1}}}"###
        );

        Ok(())
    }
}
