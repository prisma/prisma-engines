use indoc::indoc;
use query_engine_tests::*;

#[test_suite(suite = "cascade_onD_1to1_req", schema(required))]
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
                parent    Parent @relation(fields: [parent_id], references: [id], onDelete: Cascade)
            }"#
        };

        schema.to_owned()
    }

    /// Deleting the parent deletes child as well.
    #[connector_test]
    async fn delete_parent(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(runner, r#"mutation { createOneParent(data: { id: 1, child: { create: { id: 1 }}}) { id }}"#),
            @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, "mutation { deleteOneParent(where: { id: 1 }) { id }}"),
            @r###"{"data":{"deleteOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, "query { findManyChild { id }}"),
            @r###"{"data":{"findManyChild":[]}}"###
        );

        Ok(())
    }
}

#[test_suite(suite = "cascade_onD_1to1_opt", schema(optional))]
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
                parent    Parent? @relation(fields: [parent_id], references: [id], onDelete: Cascade)
            }"#
        };

        schema.to_owned()
    }

    /// Deleting the parent deletes child as well.
    #[connector_test]
    async fn delete_parent(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { createOneParent(data: { id: 1, child: { create: { id: 1 }}}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, "mutation { deleteOneParent(where: { id: 1 }) { id }}"),
            @r###"{"data":{"deleteOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, "query { findManyChild { id }}"),
            @r###"{"data":{"findManyChild":[]}}"###
        );

        Ok(())
    }
}

#[test_suite(suite = "cascade_onD_1toM_req", schema(required))]
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
                parent    Parent @relation(fields: [parent_id], references: [id], onDelete: Cascade)
            }"#
        };

        schema.to_owned()
    }

    /// Deleting the parent deletes all children.
    #[connector_test]
    async fn delete_parent(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { createOneParent(data: { id: 1, children: { create: [ { id: 1 }, { id: 2 } ] }}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, "mutation { deleteOneParent(where: { id: 1 }) { id }}"),
            @r###"{"data":{"deleteOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, "query { findManyChild { id }}"),
            @r###"{"data":{"findManyChild":[]}}"###
        );

        Ok(())
    }
}

#[test_suite(suite = "cascade_onD_1toM_opt", schema(optional))]
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
                parent    Parent? @relation(fields: [parent_id], references: [id], onDelete: Cascade)
            }"#
        };

        schema.to_owned()
    }

    /// Deleting the parent deletes all children.
    #[connector_test]
    async fn delete_parent(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation { createOneParent(data: { id: 1, children: { create: [ { id: 1 }, { id: 2 } ] }}) { id }}"#),
          @r###"{"data":{"createOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, "mutation { deleteOneParent(where: { id: 1 }) { id }}"),
            @r###"{"data":{"deleteOneParent":{"id":1}}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, "query { findManyChild { id }}"),
            @r###"{"data":{"findManyChild":[]}}"###
        );

        Ok(())
    }
}
