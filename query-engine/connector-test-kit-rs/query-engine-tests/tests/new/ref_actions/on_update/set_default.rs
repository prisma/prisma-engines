//! MySQL doesn't support SetDefault for InnoDB (which is our only supported engine at the moment).
use indoc::indoc;
use query_engine_tests::*;

#[test_suite(suite = "setdefault_onU_1to1_req", exclude(MongoDb, MySQL))]
mod one2one_req {
    fn required_with_default() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                uniq  String @unique
                child Child?
            }

            model Child {
                #id(id, Int, @id)
                parent_uniq String  @default("2") @unique
                parent      Parent @relation(fields: [parent_uniq], references: [uniq], onUpdate: SetDefault)
            }"#
        };

        schema.to_owned()
    }

    fn required_without_default() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                uniq  String @unique
                child Child?
            }

            model Child {
                #id(id, Int, @id)
                parent_uniq String @unique
                parent    Parent @relation(fields: [parent_uniq], references: [uniq], onUpdate: SetDefault)
            }"#
        };

        schema.to_owned()
    }

    /// Updating the parent reconnects the child to the default.
    #[connector_test(schema(required_with_default))]
    async fn update_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        // The default
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 2, uniq: "2" }) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":2}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#),
          @r###"{"data":{"updateOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyChild { id parent { id } }}"#),
          @r###"{"data":{"findManyChild":[{"id":1,"parent":{"id":2}}]}}"###
        );

        Ok(())
    }

    /// Updating the parent reconnects the child to the default and fails (the default doesn't exist).
    #[connector_test(schema(required_with_default), exclude(MongoDb, MySQL))]
    async fn update_parent_no_exist_fail(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        assert_error!(
            &runner,
            r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#,
            2003,
            "Foreign key constraint failed on the field"
        );

        Ok(())
    }

    /// Updating the parent with no default for SetDefault fails.
    /// Only postgres (except CockroachDB) allows setting no default for a SetDefault FK.
    #[connector_test(schema(required_without_default), only(Postgres), exclude(CockroachDb))]
    async fn update_parent_fail(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        assert_error!(
            &runner,
            r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#,
            2011,
            "Null constraint violation on the fields"
        );

        Ok(())
    }
}

#[test_suite(suite = "setdefault_onU_1to1_opt", exclude(MongoDb, MySQL))]
mod one2one_opt {
    fn optional_with_default() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                uniq  String @unique
                child Child?
            }

            model Child {
                #id(id, Int, @id)
                parent_uniq String?    @default("2") @unique
                parent    Parent? @relation(fields: [parent_uniq], references: [uniq], onUpdate: SetDefault)
            }"#
        };

        schema.to_owned()
    }

    fn optional_without_default() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                uniq  String @unique
                child Child?
            }

            model Child {
                #id(id, Int, @id)
                parent_uniq String? @unique
                parent    Parent? @relation(fields: [parent_uniq], references: [uniq], onUpdate: SetDefault)
            }"#
        };

        schema.to_owned()
    }

    /// Updating the parent reconnects the child to the default.
    #[connector_test(schema(optional_with_default))]
    async fn update_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        // The default
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 2, uniq: "2" }) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":2}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#),
          @r###"{"data":{"updateOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyChild { id parent { id } }}"#),
          @r###"{"data":{"findManyChild":[{"id":1,"parent":{"id":2}}]}}"###
        );

        Ok(())
    }

    /// Updating the parent reconnects the child to the default and fails (the default doesn't exist).
    #[connector_test(schema(optional_with_default), exclude(MongoDb, MySQL))]
    async fn update_parent_no_exist_fail(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        assert_error!(
            &runner,
            r#"mutation { updateOneParent(where: { id: 1 } data: { uniq: "u1" }) { id }}"#,
            2003,
            "Foreign key constraint failed on the field"
        );

        Ok(())
    }

    /// Updating the parent with no default for SetDefault nulls the FK.
    #[connector_test(schema(optional_without_default), only(Postgres))]
    async fn update_parent_fail(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#),
          @r###"{"data":{"updateOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyChild(where: { id: 1 }) { id parent_uniq }}"#),
          @r###"{"data":{"findManyChild":[{"id":1,"parent_uniq":null}]}}"###
        );

        Ok(())
    }
}

#[test_suite(suite = "setdefault_onU_1toM_req", exclude(MongoDb, MySQL))]
mod one2many_req {
    fn required_with_default() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                uniq     String @unique
                children Child[]
            }

            model Child {
                #id(id, Int, @id)
                parent_uniq String @default("2")
                parent      Parent @relation(fields: [parent_uniq], references: [uniq], onUpdate: SetDefault)
            }"#
        };

        schema.to_owned()
    }

    fn required_without_default() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                uniq     String @unique
                children Child[]
            }

            model Child {
                #id(id, Int, @id)
                parent_uniq String
                parent      Parent @relation(fields: [parent_uniq], references: [uniq], onUpdate: SetDefault)
            }"#
        };

        schema.to_owned()
    }

    /// Updating the parent reconnects the children to the default.
    #[connector_test(schema(required_with_default))]
    async fn update_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        // The default
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 2, uniq: "2" }) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":2}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#),
          @r###"{"data":{"updateOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyChild { id parent { id } }}"#),
          @r###"{"data":{"findManyChild":[{"id":1,"parent":{"id":2}}]}}"###
        );

        Ok(())
    }

    /// Updating the parent reconnects the child to the default and fails (the default doesn't exist).
    #[connector_test(schema(required_with_default), exclude(MongoDb, MySQL))]
    async fn update_parent_no_exist_fail(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        assert_error!(
            &runner,
            r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#,
            2003,
            "Foreign key constraint failed on the field"
        );

        Ok(())
    }

    /// Updating the parent with no default for SetDefault fails.
    /// Only postgres (except CockroachDB) allows setting no default for a SetDefault FK.
    #[connector_test(schema(required_without_default), only(Postgres), exclude(CockroachDb))]
    async fn update_parent_fail(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        assert_error!(
            &runner,
            r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#,
            2011,
            "Null constraint violation on the fields"
        );

        Ok(())
    }
}

#[test_suite(suite = "setdefault_onU_1toM_opt", exclude(MongoDb, MySQL))]
mod one2many_opt {
    fn optional_with_default() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                uniq     String @unique
                children Child[]
            }

            model Child {
                #id(id, Int, @id)
                parent_uniq String? @default("2")
                parent      Parent? @relation(fields: [parent_uniq], references: [uniq], onUpdate: SetDefault)
            }"#
        };

        schema.to_owned()
    }

    fn optional_without_default() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                uniq     String @unique
                children Child[]
            }

            model Child {
                #id(id, Int, @id)
                parent_uniq String?
                parent      Parent? @relation(fields: [parent_uniq], references: [uniq], onUpdate: SetDefault)
            }"#
        };

        schema.to_owned()
    }

    /// Updating the parent reconnects the child to the default.
    #[connector_test(schema(optional_with_default))]
    async fn update_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        // The default
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 2, uniq: "2" }) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":2}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#),
          @r###"{"data":{"updateOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyChild { id parent { id } }}"#),
          @r###"{"data":{"findManyChild":[{"id":1,"parent":{"id":2}}]}}"###
        );

        Ok(())
    }

    /// Updating the parent reconnects the child to the default and fails (the default doesn't exist).
    #[connector_test(schema(optional_with_default), exclude(MongoDb, MySQL))]
    async fn update_parent_no_exist_fail(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        assert_error!(
            &runner,
            r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#,
            2003,
            "Foreign key constraint failed on the field"
        );

        Ok(())
    }

    /// Updating the parent with no default for SetDefault nulls the FK.
    #[connector_test(schema(optional_without_default), only(Postgres))]
    async fn update_parent_fail(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", children: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "u1" }) { id }}"#),
          @r###"{"data":{"updateOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyChild(where: { id: 1 }) { id parent_uniq }}"#),
          @r###"{"data":{"findManyChild":[{"id":1,"parent_uniq":null}]}}"###
        );

        Ok(())
    }
}
