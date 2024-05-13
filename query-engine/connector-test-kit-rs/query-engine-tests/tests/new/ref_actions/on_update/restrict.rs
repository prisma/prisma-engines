//! SQL Server doesn't support Restrict.
//! D1 seems to silently ignore Restrict.

use indoc::indoc;
use query_engine_tests::*;

#[test_suite(
    suite = "restrict_onU_1to1_req",
    schema(required),
    exclude(SqlServer),
    relation_mode = "prisma"
)]
mod one2one_req {
    fn required() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                uniq  String @unique
                name String
                child Child?
            }

            model Child {
                #id(id, Int, @id)
                parent_uniq String @unique
                parent      Parent @relation(fields: [parent_uniq], references: [uniq], onUpdate: Restrict)
            }"#
        };

        schema.to_owned()
    }

    /// Updating the parent must fail if a child is connected.
    #[connector_test]
    async fn update_parent_failure(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let query = r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#;

        assert_error!(
            runner,
            query,
            2014,
            "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
        );

        Ok(())
    }

    /// Updating the parent must fail if a child is connected.
    #[connector_test]
    async fn update_many_parent_failure(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let query = r#"mutation { updateManyParent(where: { id: 1 }, data: { uniq: "u1" }) { count }}"#;

        assert_error!(
            runner,
            query,
            2014,
            "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
        );

        Ok(())
    }

    /// Updating the parent must fail if a child is connected.
    #[connector_test]
    async fn upsert_parent_failure(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let query = r#"mutation { upsertOneParent(where: { id: 1 }, update: { uniq: "u1" }, create: { id: 1, name: "Bob", uniq: "1", child: { create: { id: 1 }} }) { id }}"#;

        assert_error!(
            runner,
            query,
            2014,
            "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation { createOneParent(data: { id: 1, name: "Bob", uniq: "1", child: { create: { id: 1 }}}) { id }}"#
        );

        Ok(())
    }
}

#[test_suite(
    suite = "restrict_onU_1to1_opt",
    schema(optional),
    exclude(SqlServer),
    relation_mode = "prisma"
)]
mod one2one_opt {
    fn optional() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                uniq  String @unique
                name String
                child Child?
            }

            model Child {
                #id(id, Int, @id)
                parent_uniq String? @unique
                parent      Parent? @relation(fields: [parent_uniq], references: [uniq], onUpdate: Restrict)
            }"#
        };

        schema.to_owned()
    }

    /// Updating the parent must fail if a child is connected.
    #[connector_test]
    async fn update_parent_failure(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let query = r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#;

        assert_error!(
            runner,
            query,
            2014,
            "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
        );

        Ok(())
    }

    /// Updating the parent must fail if a child is connected.
    #[connector_test]
    async fn update_many_parent_failure(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let query = r#"mutation { updateManyParent(where: { id: 1 }, data: { uniq: "u1" }) { count }}"#;

        assert_error!(
            runner,
            query,
            2014,
            "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
        );

        Ok(())
    }

    /// Updating the parent must fail if a child is connected.
    #[connector_test]
    async fn upsert_parent_failure(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let query = r#"mutation { upsertOneParent(where: { id: 1 }, update: { uniq: "u1" }, create: { id: 1, name: "Bob", uniq: "1", child: { create: { id: 1 }} }) { id }}"#;

        assert_error!(
            runner,
            query,
            2014,
            "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation { createOneParent(data: { id: 1, name: "Bob", uniq: "1", child: { create: { id: 1 }}}) { id }}"#
        );

        Ok(())
    }
}

#[test_suite(
    suite = "restrict_onU_1toM_req",
    schema(required),
    exclude(SqlServer),
    relation_mode = "prisma"
)]
mod one2many_req {
    fn required() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                name String
                uniq     String @unique
                children Child[]
            }

            model Child {
                #id(id, Int, @id)
                parent_uniq String
                parent      Parent @relation(fields: [parent_uniq], references: [uniq], onUpdate: Restrict)
            }"#
        };

        schema.to_owned()
    }

    /// Updating the parent must fail if a child is connected.
    #[connector_test]
    async fn update_parent_failure(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let query = r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#;

        assert_error!(
            runner,
            query,
            2014,
            "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
        );

        Ok(())
    }

    /// Updating the parent must fail if a child is connected.
    #[connector_test]
    async fn update_many_parent_failure(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let query = r#"mutation { updateManyParent(where: { id: 1 }, data: { uniq: "u1" }) { count }}"#;

        assert_error!(
            runner,
            query,
            2014,
            "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
        );

        Ok(())
    }

    /// Updating the parent must fail if a child is connected.
    #[connector_test]
    async fn upsert_parent_failure(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let query = r#"mutation { upsertOneParent(where: { id: 1 }, update: { uniq: "u1" }, create: { id: 1, name: "Bob", uniq: "1", children: { create: { id: 1 }} }) { id }}"#;

        assert_error!(
            runner,
            query,
            2014,
            "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
        );

        Ok(())
    }

    #[connector_test(exclude(Sqlite("cfd1")))]
    /// Updating the parent succeeds if no child is connected or if the linking fields aren't part of the update payload.
    ///
    /// ```diff
    /// - {"data":{"updateManyParent":{"count":1}}}
    /// + {"data":{"updateManyParent":{"count":2}}}
    /// ```
    async fn update_parent(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;
        run_query!(
            &runner,
            r#"mutation { createOneParent(data: { id: 2, name: "Bob2", uniq: "2" }) { id }}"#
        );

        // Linking field updated but no child connected: works
        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { updateOneParent(where: { id: 2 }, data: { uniq: "u2" }) { id }}"#),
            @r###"{"data":{"updateOneParent":{"id":2}}}"###
        );

        // Child connected but no linking field updated: works
        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { name: "Alice" }) { id }}"#),
            @r###"{"data":{"updateOneParent":{"id":1}}}"###
        );

        // Linking field updated but no child connected: works
        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { updateManyParent(where: { id: 2 }, data: { uniq: "u22" }) { count }}"#),
            @r###"{"data":{"updateManyParent":{"count":1}}}"###
        );

        // No child connected and no linking field updated: works
        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { updateManyParent(where: { id: 2 }, data: { name: "Alice2" }) { count }}"#),
            @r###"{"data":{"updateManyParent":{"count":1}}}"###
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation { createOneParent(data: { id: 1, name: "Bob", uniq: "1", children: { create: { id: 1 }}}) { id }}"#
        );

        Ok(())
    }
}

#[test_suite(
    suite = "restrict_onU_1toM_opt",
    schema(optional),
    exclude(SqlServer),
    relation_mode = "prisma"
)]
mod one2many_opt {
    fn optional() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                name String
                uniq     String @unique
                children Child[]
            }

            model Child {
                #id(id, Int, @id)
                parent_uniq String?
                parent      Parent? @relation(fields: [parent_uniq], references: [uniq], onUpdate: Restrict)
            }"#
        };

        schema.to_owned()
    }

    /// Updating the parent must fail if a child is connected.
    #[connector_test]
    async fn update_parent_failure(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let query = r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#;

        assert_error!(
            runner,
            query,
            2014,
            "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
        );

        Ok(())
    }

    /// Updating the parent must fail if a child is connected.
    #[connector_test]
    async fn update_many_parent_failure(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let query = r#"mutation { updateManyParent(where: { id: 1 }, data: { uniq: "u1" }) { count }}"#;

        assert_error!(
            runner,
            query,
            2014,
            "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
        );

        Ok(())
    }

    /// Updating the parent must fail if a child is connected.
    #[connector_test]
    async fn upsert_parent_failure(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let query = r#"mutation { upsertOneParent(where: { id: 1 }, update: { uniq: "u1" }, create: { id: 1, name: "Bob", uniq: "1", children: { create: { id: 1 }} }) { id }}"#;

        assert_error!(
            runner,
            query,
            2014,
            "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
        );

        Ok(())
    }

    #[connector_test(exclude(Sqlite("cfd1")))]
    /// Updating the parent succeeds if no child is connected or if the linking fields aren't part of the update payload.
    ///
    /// ```diff
    /// - {"data":{"updateManyParent":{"count":1}}}
    /// + {"data":{"updateManyParent":{"count":2}}}
    /// ```
    async fn update_parent(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;
        run_query!(
            &runner,
            r#"mutation { createOneParent(data: { id: 2, name: "Bob2", uniq: "2" }) { id }}"#
        );

        // Linking field updated but no child connected: works
        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { updateOneParent(where: { id: 2 }, data: { uniq: "u2" }) { id }}"#),
            @r###"{"data":{"updateOneParent":{"id":2}}}"###
        );

        // Child connected but no linking field updated: works
        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { name: "Alice" }) { id }}"#),
            @r###"{"data":{"updateOneParent":{"id":1}}}"###
        );

        // Linking field updated but no child connected: works
        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { updateManyParent(where: { id: 2 }, data: { uniq: "u22" }) { count }}"#),
            @r###"{"data":{"updateManyParent":{"count":1}}}"###
        );

        // No connected and no linking field updated: works
        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { updateManyParent(where: { id: 2 }, data: { name: "Alice2" }) { count }}"#),
            @r###"{"data":{"updateManyParent":{"count":1}}}"###
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation { createOneParent(data: { id: 1, name: "Bob", uniq: "1", children: { create: { id: 1 }}}) { id }}"#
        );

        Ok(())
    }
}
