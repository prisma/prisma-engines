//! Only Postgres (except CockroachDB) allows SetNull on a non-nullable FK at all, rest fail during migration.

use indoc::indoc;
use query_engine_tests::*;

#[test_suite(suite = "setnull_onU_1to1_req", schema(required), only(Postgres), exclude(Cockroach))]
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
                parent      Parent @relation(fields: [parent_uniq], references: [uniq], onUpdate: SetNull)
            }"#
        };

        schema.to_owned()
    }

    /// Updating the parent must fail if a child is connected (because of null key violation).
    #[connector_test]
    async fn delete_parent_failure(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        //  `onUpdate: SetNull` would cause `null` on `parent_uniq`, throwing an error.
        assert_error!(
            &runner,
            r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#,
            2011,
            "Null constraint violation on the fields: (`parent_uniq`)"
        );

        assert_error!(
            &runner,
            r#"mutation { updateManyParent(where: { id: 1 }, data: { uniq: "u1" }) { count }}"#,
            2011,
            "Null constraint violation on the fields: (`parent_uniq`)"
        );

        Ok(())
    }
}

#[test_suite(suite = "setnull_onU_1to1_opt", schema(optional), exclude(MongoDb))]
mod one2one_opt {
    fn optional() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                uniq  String? @unique
                child Child?
            }

            model Child {
                #id(id, Int, @id)
                parent_uniq String?
                parent      Parent? @relation(fields: [parent_uniq], references: [uniq], onUpdate: SetNull)
            }"#
        };

        schema.to_owned()
    }

    /// Updating the parent suceeds and sets the FK null.
    #[connector_test]
    async fn delete_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#),
          @r###"{"data":{"updateOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyChild { id parent_uniq }}"#),
          @r###"{"data":{"findManyChild":[{"id":1,"parent_uniq":null}]}}"###
        );

        Ok(())
    }
}

#[test_suite(suite = "setnull_onU_1toM_req", schema(required), only(Postgres), exclude(Cockroach))]
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
                parent    Parent @relation(fields: [parent_uniq], references: [uniq], onUpdate: SetNull)
            }"#
        };

        schema.to_owned()
    }

    /// Updating the parent must fail if a child is connected (because of null key violation).
    #[connector_test]
    async fn delete_parent_failure(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        //  `onUpdate: SetNull` would cause `null` on `parent_uniq`, throwing an error.
        assert_error!(
            &runner,
            r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#,
            2011,
            "Null constraint violation on the fields: (`parent_uniq`)"
        );

        assert_error!(
            &runner,
            r#"mutation { updateManyParent(where: { id: 1 }, data: { uniq: "u1" }) { count }}"#,
            2011,
            "Null constraint violation on the fields: (`parent_uniq`)"
        );

        Ok(())
    }
}

#[test_suite(suite = "setnull_onU_1toM_opt", schema(optional), exclude(MongoDb))]
mod one2many_opt {
    fn optional() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                uniq     String? @unique
                children Child[]
            }

            model Child {
                #id(id, Int, @id)
                parent_uniq String?
                parent    Parent? @relation(fields: [parent_uniq], references: [uniq], onUpdate: SetNull)
            }"#
        };

        schema.to_owned()
    }

    /// Updating the parent suceeds and sets the FK null.
    #[connector_test]
    async fn delete_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#),
          @r###"{"data":{"updateOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyChild { id parent_uniq }}"#),
          @r###"{"data":{"findManyChild":[{"id":1,"parent_uniq":null}]}}"###
        );

        Ok(())
    }
}
