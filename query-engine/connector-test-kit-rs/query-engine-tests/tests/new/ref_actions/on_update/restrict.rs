//! SQL Server doesn't support Restrict.

use indoc::indoc;
use query_engine_tests::*;

#[test_suite(suite = "restrict_onU_1to1_req", schema(required), exclude(SqlServer))]
mod one2one_req {
    fn required() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                uniq  String @unique
                child Child?
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
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        let query = r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#;

        match runner.connector() {
            ConnectorTag::MongoDb(_) => assert_error!(
                runner,
                query,
                2014,
                "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
            ),
            _ => assert_error!(
                runner,
                query,
                2003,
                "Foreign key constraint failed on the field"
            ),
        };

        Ok(())
    }

    /// Updating the parent must fail if a child is connected.
    #[connector_test]
    async fn update_many_parent_failure(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        let query = r#"mutation { updateManyParent(where: { id: 1 }, data: { uniq: "u1" }) { count }}"#;

        match runner.connector() {
            ConnectorTag::MongoDb(_) => assert_error!(
                runner,
                query,
                2014,
                "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
            ),
            _ => assert_error!(
                runner,
                query,
                2003,
                "Foreign key constraint failed on the field"
            ),
        };

        Ok(())
    }

    /// Updating the parent must fail if a child is connected.
    #[connector_test]
    async fn upsert_parent_failure(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        let query = r#"mutation { upsertOneParent(where: { id: 1 }, update: { uniq: "u1" }, create: { id: 1, uniq: "1", child: { create: { id: 1 }} }) { id }}"#;

        match runner.connector() {
            ConnectorTag::MongoDb(_) => assert_error!(
                runner,
                query,
                2014,
                "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
            ),
            _ => assert_error!(
                runner,
                query,
                2003,
                "Foreign key constraint failed on the field"
            ),
        };

        Ok(())
    }
}

#[test_suite(suite = "restrict_onU_1to1_opt", schema(optional), exclude(SqlServer))]
mod one2one_opt {
    fn optional() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                uniq  String @unique
                child Child?
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
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        let query = r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#;

        match runner.connector() {
            ConnectorTag::MongoDb(_) => assert_error!(
                runner,
                query,
                2014,
                "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
            ),
            _ => assert_error!(
                runner,
                query,
                2003,
                "Foreign key constraint failed on the field"
            ),
        };

        Ok(())
    }

    /// Updating the parent must fail if a child is connected.
    #[connector_test]
    async fn update_many_parent_failure(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        let query = r#"mutation { updateManyParent(where: { id: 1 }, data: { uniq: "u1" }) { count }}"#;

        match runner.connector() {
            ConnectorTag::MongoDb(_) => assert_error!(
                runner,
                query,
                2014,
                "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
            ),
            _ => assert_error!(
                runner,
                query,
                2003,
                "Foreign key constraint failed on the field"
            ),
        };

        Ok(())
    }

    /// Updating the parent must fail if a child is connected.
    #[connector_test]
    async fn upsert_parent_failure(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        let query = r#"mutation { upsertOneParent(where: { id: 1 }, update: { uniq: "u1" }, create: { id: 1, uniq: "1", child: { create: { id: 1 }} }) { id }}"#;

        match runner.connector() {
            ConnectorTag::MongoDb(_) => assert_error!(
                runner,
                query,
                2014,
                "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
            ),
            _ => assert_error!(
                runner,
                query,
                2003,
                "Foreign key constraint failed on the field"
            ),
        };

        Ok(())
    }

    /// Updating the parent succeeds if no child is connected.
    #[connector_test]
    async fn update_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1" }) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 2, uniq: "2" }) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":2}}}"###
        );

        insta::assert_snapshot!(
            r#run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#),
            @r###"{"data":{"updateOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { updateManyParent(where: { id: 2 }, data: { uniq: "u2" }) { count }}"#),
            @r###"{"data":{"updateManyParent":{"count":1}}}"###
        );

        Ok(())
    }
}

#[test_suite(suite = "restrict_onU_1toM_req", schema(required), exclude(SqlServer))]
mod one2many_req {
    fn required() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
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
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        let query = r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#;

        match runner.connector() {
            ConnectorTag::MongoDb(_) => assert_error!(
                runner,
                query,
                2014,
                "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
            ),
            _ => assert_error!(
                runner,
                query,
                2003,
                "Foreign key constraint failed on the field"
            ),
        };

        Ok(())
    }

    /// Updating the parent must fail if a child is connected.
    #[connector_test]
    async fn update_many_parent_failure(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        let query = r#"mutation { updateManyParent(where: { id: 1 }, data: { uniq: "u1" }) { count }}"#;

        match runner.connector() {
            ConnectorTag::MongoDb(_) => assert_error!(
                runner,
                query,
                2014,
                "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
            ),
            _ => assert_error!(
                runner,
                query,
                2003,
                "Foreign key constraint failed on the field"
            ),
        };

        Ok(())
    }

    /// Updating the parent must fail if a child is connected.
    #[connector_test]
    async fn upsert_parent_failure(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        let query = r#"mutation { upsertOneParent(where: { id: 1 }, update: { uniq: "u1" }, create: { id: 1, uniq: "1", children: { create: { id: 1 }} }) { id }}"#;

        match runner.connector() {
            ConnectorTag::MongoDb(_) => assert_error!(
                runner,
                query,
                2014,
                "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
            ),
            _ => assert_error!(
                runner,
                query,
                2003,
                "Foreign key constraint failed on the field"
            ),
        };

        Ok(())
    }

    /// Updating the parent succeeds if no child is connected.
    #[connector_test]
    async fn update_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1" }) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 2, uniq: "2" }) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":2}}}"###
        );

        insta::assert_snapshot!(
            r#run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#),
            @r###"{"data":{"updateOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { updateManyParent(where: { id: 2 }, data: { uniq: "u2" }) { count }}"#),
            @r###"{"data":{"updateManyParent":{"count":1}}}"###
        );

        Ok(())
    }
}

#[test_suite(suite = "restrict_onU_1toM_opt", schema(optional), exclude(SqlServer))]
mod one2many_opt {
    fn optional() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
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
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, , uniq: "1", children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        let query = r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#;

        match runner.connector() {
            ConnectorTag::MongoDb(_) => assert_error!(
                runner,
                query,
                2014,
                "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
            ),
            _ => assert_error!(
                runner,
                query,
                2003,
                "Foreign key constraint failed on the field"
            ),
        };

        Ok(())
    }

    /// Updating the parent must fail if a child is connected.
    #[connector_test]
    async fn update_many_parent_failure(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        let query = r#"mutation { updateManyParent(where: { id: 1 }, data: { uniq: "u1" }) { count }}"#;

        match runner.connector() {
            ConnectorTag::MongoDb(_) => assert_error!(
                runner,
                query,
                2014,
                "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
            ),
            _ => assert_error!(
                runner,
                query,
                2003,
                "Foreign key constraint failed on the field"
            ),
        };

        Ok(())
    }

    /// Updating the parent must fail if a child is connected.
    #[connector_test]
    async fn upsert_parent_failure(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        let query = r#"mutation { upsertOneParent(where: { id: 1 }, update: { uniq: "u1" }, create: { id: 1, uniq: "1", children: { create: { id: 1 }} }) { id }}"#;

        match runner.connector() {
            ConnectorTag::MongoDb(_) => assert_error!(
                runner,
                query,
                2014,
                "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
            ),
            _ => assert_error!(
                runner,
                query,
                2003,
                "Foreign key constraint failed on the field"
            ),
        };

        Ok(())
    }

    /// Updating the parent succeeds if no child is connected.
    #[connector_test]
    async fn update_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1" }) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 2, uniq: "2" }) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":2}}}"###
        );

        insta::assert_snapshot!(
            r#run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#),
            @r###"{"data":{"updateOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { updateManyParent(where: { id: 2 }, data: { uniq: "u2" }) { count }}"#),
            @r###"{"data":{"updateManyParent":{"count":1}}}"###
        );

        Ok(())
    }
}
