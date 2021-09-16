use indoc::indoc;
use query_engine_tests::*;

#[test_suite(suite = "cascade_onU_1to1_req", schema(required), exclude(MongoDb))]
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
                parent    Parent @relation(fields: [parent_uniq], references: [uniq], onUpdate: Cascade)
            }"#
        };

        schema.to_owned()
    }

    /// Updating the parent updates the child as well.
    #[connector_test]
    async fn update_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", child: { create: { id: 1 }}}) { id }}"#),
            @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "1u" }) { uniq }}"#),
            @r###"{"data":{"updateOneParent":{"uniq":"1u"}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { findManyParent { uniq child { parent_uniq } }}"),
            @r###"{"data":{"findManyParent":[{"uniq":"1u","child":{"parent_uniq":"1u"}}]}}"###
        );

        Ok(())
    }
}

#[test_suite(suite = "cascade_onU_1to1_opt", schema(optional), exclude(MongoDb))]
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
                parent      Parent? @relation(fields: [parent_uniq], references: [uniq], onUpdate: Cascade)
            }"#
        };

        schema.to_owned()
    }

    /// Updating the parent updates the child as well.
    #[connector_test]
    async fn update_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", child: { create: { id: 1 }}}) { id }}"#),
            @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "1u" }) { uniq }}"#),
            @r###"{"data":{"updateOneParent":{"uniq":"1u"}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { findManyParent { uniq child { parent_uniq } }}"),
            @r###"{"data":{"findManyParent":[{"uniq":"1u","child":{"parent_uniq":"1u"}}]}}"###
        );

        Ok(())
    }
}

#[test_suite(suite = "cascade_onU_1toM_req", schema(required), exclude(MongoDb))]
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
                parent    Parent @relation(fields: [parent_uniq], references: [uniq], onUpdate: Cascade)
            }"#
        };

        schema.to_owned()
    }

    /// Updating the parent updates the child as well.
    #[connector_test]
    async fn update_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", children: { create: { id: 1 }}}) { id }}"#),
            @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "1u" }) { uniq }}"#),
            @r###"{"data":{"updateOneParent":{"uniq":"1u"}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { findManyParent { uniq children { parent_uniq } }}"),
            @r###"{"data":{"findManyParent":[{"uniq":"1u","children":[{"parent_uniq":"1u"}]}]}}"###
        );

        Ok(())
    }
}

#[test_suite(suite = "cascade_onU_1toM_opt", schema(optional), exclude(MongoDb))]
mod one2many_opt {
    fn optional() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)
                uniq     String  @unique
                children Child[]
            }

            model Child {
                #id(id, Int, @id)
                parent_uniq String?
                parent    Parent? @relation(fields: [parent_uniq], references: [uniq], onUpdate: Cascade)
            }"#
        };

        schema.to_owned()
    }

    /// Updating the parent updates the child as well.
    #[connector_test]
    async fn update_parent(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { createOneParent(data: { id: 1, uniq: "1", children: { create: { id: 1 }}}) { id }}"#),
            @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { updateOneParent(where: { id: 1 }, data: { uniq: "1u" }) { uniq }}"#),
            @r###"{"data":{"updateOneParent":{"uniq":"1u"}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { findManyParent { uniq children { parent_uniq } }}"),
            @r###"{"data":{"findManyParent":[{"uniq":"1u","children":[{"parent_uniq":"1u"}]}]}}"###
        );

        Ok(())
    }
}
