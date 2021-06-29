//! SQL Server doesn't support Restrict.

use indoc::indoc;
use query_engine_tests::*;

#[test_suite(suite = "restrict_onD_1to1_req", schema(required), exclude(SqlServer))]
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
                parent    Parent @relation(fields: [parent_id], references: [id], onDelete: Restrict)
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

        match runner.connector() {
            ConnectorTag::MongoDb(_) => assert_error!(
                runner,
                "mutation { deleteOneParent(where: { id: 1 }) { id }}",
                2014,
                "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
            ),
            ConnectorTag::Sqlite(_) => insta::assert_snapshot!(
                run_query!(runner, r#"mutation { deleteOneParent(where: { id: 1 }) { id }}"#),
                @r###"{"errors":[{"error":"Error occurred during query execution:\nConnectorError(ConnectorError { user_facing_error: None, kind: QueryError(SqliteFailure(Error { code: ConstraintViolation, extended_code: 1811 }, Some(\"FOREIGN KEY constraint failed\"))) })","user_facing_error":{"is_panic":false,"message":"Error occurred during query execution:\nConnectorError(ConnectorError { user_facing_error: None, kind: QueryError(SqliteFailure(Error { code: ConstraintViolation, extended_code: 1811 }, Some(\"FOREIGN KEY constraint failed\"))) })","backtrace":null}}]}"###
            ),
            _ => assert_error!(
                runner,
                "mutation { deleteOneParent(where: { id: 1 }) { id }}",
                2003,
                "Foreign key constraint failed on the field"
            ),
        };

        Ok(())
    }
}

#[test_suite(suite = "restrict_onD_1to1_opt", schema(optional), exclude(SqlServer))]
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
                parent    Parent? @relation(fields: [parent_id], references: [id], onDelete: Restrict)
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

        match runner.connector() {
            ConnectorTag::MongoDb(_) => assert_error!(
                runner,
                "mutation { deleteOneParent(where: { id: 1 }) { id }}",
                2014,
                "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
            ),
            ConnectorTag::Sqlite(_) => insta::assert_snapshot!(
                run_query!(runner, r#"mutation { deleteOneParent(where: { id: 1 }) { id }}"#),
                @r###"{"errors":[{"error":"Error occurred during query execution:\nConnectorError(ConnectorError { user_facing_error: None, kind: QueryError(SqliteFailure(Error { code: ConstraintViolation, extended_code: 1811 }, Some(\"FOREIGN KEY constraint failed\"))) })","user_facing_error":{"is_panic":false,"message":"Error occurred during query execution:\nConnectorError(ConnectorError { user_facing_error: None, kind: QueryError(SqliteFailure(Error { code: ConstraintViolation, extended_code: 1811 }, Some(\"FOREIGN KEY constraint failed\"))) })","backtrace":null}}]}"###
            ),
            _ => assert_error!(
                runner,
                "mutation { deleteOneParent(where: { id: 1 }) { id }}",
                2003,
                "Foreign key constraint failed on the field"
            ),
        };

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
}

#[test_suite(suite = "restrict_onD_1toM_req", schema(required), exclude(SqlServer))]
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
    async fn delete_parent_failure(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { createOneParent(data: { id: 1, children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        match runner.connector() {
            ConnectorTag::MongoDb(_) => assert_error!(
                runner,
                "mutation { deleteOneParent(where: { id: 1 }) { id }}",
                2014,
                "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
            ),
            ConnectorTag::Sqlite(_) => insta::assert_snapshot!(
                run_query!(runner, r#"mutation { deleteOneParent(where: { id: 1 }) { id }}"#),
                @r###"{"errors":[{"error":"Error occurred during query execution:\nConnectorError(ConnectorError { user_facing_error: None, kind: QueryError(SqliteFailure(Error { code: ConstraintViolation, extended_code: 1811 }, Some(\"FOREIGN KEY constraint failed\"))) })","user_facing_error":{"is_panic":false,"message":"Error occurred during query execution:\nConnectorError(ConnectorError { user_facing_error: None, kind: QueryError(SqliteFailure(Error { code: ConstraintViolation, extended_code: 1811 }, Some(\"FOREIGN KEY constraint failed\"))) })","backtrace":null}}]}"###
            ),
            _ => assert_error!(
                runner,
                "mutation { deleteOneParent(where: { id: 1 }) { id }}",
                2003,
                "Foreign key constraint failed on the field"
            ),
        };

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
}

#[test_suite(suite = "restrict_onD_1toM_opt", schema(optional), exclude(SqlServer))]
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
    async fn delete_parent_failure(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { createOneParent(data: { id: 1, children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        match runner.connector() {
            ConnectorTag::MongoDb(_) => assert_error!(
                runner,
                "mutation { deleteOneParent(where: { id: 1 }) { id }}",
                2014,
                "The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."
            ),
            ConnectorTag::Sqlite(_) => insta::assert_snapshot!(
                run_query!(runner, r#"mutation { deleteOneParent(where: { id: 1 }) { id }}"#),
                @r###"{"errors":[{"error":"Error occurred during query execution:\nConnectorError(ConnectorError { user_facing_error: None, kind: QueryError(SqliteFailure(Error { code: ConstraintViolation, extended_code: 1811 }, Some(\"FOREIGN KEY constraint failed\"))) })","user_facing_error":{"is_panic":false,"message":"Error occurred during query execution:\nConnectorError(ConnectorError { user_facing_error: None, kind: QueryError(SqliteFailure(Error { code: ConstraintViolation, extended_code: 1811 }, Some(\"FOREIGN KEY constraint failed\"))) })","backtrace":null}}]}"###
              ),
            _ => assert_error!(
                runner,
                "mutation { deleteOneParent(where: { id: 1 }) { id }}",
                2003,
                "Foreign key constraint failed on the field"
            ),
        };

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
}
