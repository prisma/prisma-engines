use query_engine_tests::*;

// Note: "Ported" is actually refering to the port from Graphcool to Prisma 1. This is the port of the port.
#[test_suite(schema(schema), capabilities(Enums))]
mod ported {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"
            model ModelA {
                #id(id, String, @id, @default(cuid()))
                idTest      String?
                optString   String?
                optInt      Int?
                optFloat    Float?
                optBoolean  Boolean?
                optDateTime DateTime?
                optEnum     Enum?
                b_id        String?
                b           ModelB?        @relation(fields: [b_id], references: [id])
            }

            model ModelB {
                #id(id, String, @id, @default(cuid()))
                int Int? @unique
                m   ModelA []
            }

            enum Enum{
               A
               B
            }
            "#
        };

        schema.to_owned()
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn l1_and(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;
        create_model_a(&runner, "id1", Some("bar"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("foo bar"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("foo"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id4", Some("foo bar barz"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id5", Some("foo bar barz"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"
                query {
                    findManyModelA(
                        where: {
                            optString: { startsWith: "foo" }
                            optBoolean: { equals: false }
                            idTest: { endsWith: "5" }
                        }
                    ) {
                        idTest
                    }
                }
            "#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id5"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"
                query {
                    findManyModelA(
                        where: {
                            b: { is: { int: { equals: 1 } } }
                            AND: [
                                { optString: { startsWith: "foo" }},
                                { optBoolean: { equals: false }},
                                { idTest: { endsWith: "5" }}
                            ]
                        }
                    ) {
                        idTest
                    }
                }
            "#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id5"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn l2_and(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;
        create_model_a(&runner, "id1", Some("bar"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("foo bar"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("foo"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id4", Some("foo bar barz"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id5", Some("foo bar barz"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id6", Some("foo bar barz"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"
                query {
                    findManyModelA(
                        where: {
                            AND: [
                                {
                                    optBoolean: { equals: false }
                                    idTest: { endsWith: "5" }
                                    AND: [{ optString: { startsWith: "foo" } }]
                                }
                            ]
                        }
                    ) {
                        idTest
                    }
                  }
            "#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id5"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"
                query {
                    findManyModelA(
                      where: {
                        b: { is: { int: { equals: 1 } } }
                        AND: [
                          {
                            optBoolean: { equals: false }
                            idTest: { endsWith: "5" }
                            AND: [{ optString: { startsWith: "foo" } }]
                          }
                        ]
                      }
                    ) {
                      idTest
                    }
                }
            "#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id5"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn l1_or(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;
        create_model_a(&runner, "id1", Some("bar"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("foo bar"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("foo"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id4", Some("foo bar barz"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id5", Some("foo bar barz"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"
                query {
                    findManyModelA(
                        where: {
                            optBoolean: { equals: false }
                            OR: [
                                { optString: { startsWith: "foo" } }
                                { idTest: { endsWith: "5" } }
                            ]
                        }
                        orderBy: { id: asc }
                    ) {
                      idTest
                    }
                }
            "#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"},{"idTest":"id4"},{"idTest":"id5"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn l2_or(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;
        create_model_a(&runner, "id1", Some("bar"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("foo bar"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("foo"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id4", Some("foo bar barz"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id5", Some("foo bar barz"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id6", Some("foo bar barz"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"
                query {
                    findManyModelA(
                        where: {
                            OR: [
                                {
                                    optString: { startsWith: "foo" }
                                    OR: [
                                        { optBoolean: { equals: false } }
                                        { idTest: { endsWith: "5" } }
                                    ]
                                }
                            ]
                        }
                        orderBy: { id: asc }
                    ) {
                        idTest
                    }
                }
            "#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"},{"idTest":"id4"},{"idTest":"id5"},{"idTest":"id6"}]}}"###
        );

        Ok(())
    }

    // Null tests
    #[rustfmt::skip]
    #[connector_test]
    async fn filter_null(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", None, 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("foo bar"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", None, 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{ findManyModelA(where: { optString: { equals: null }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{ findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optString: { equals: null }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{ findManyModelA(where: { optString: { not: { equals: null }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optString: { not: null }}, orderBy: { id: asc }){ idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optString: { not: { not: { not: { equals: null }}}}}, orderBy: { id: asc }){ idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optString: { not: { not: { not: { equals: null }}}}}, orderBy: { id: asc }){ idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optString: { not: { equals: null }}}, orderBy: { id: asc }){ idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optString: { in: null }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optString: { in: null }}, orderBy: { id: asc }){ idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optString: { not: { in: null }}}, orderBy: { id: asc }){ idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optString: { not: { in: null }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"}]}}"###
        );

        Ok(())
    }

    // String tests
    #[rustfmt::skip]
    #[connector_test]
    async fn str_eq(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("bar"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("foo bar"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("foo bar barz"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optString: { equals: "bar" }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optString: { equals: "bar" }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn str_not_eq(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("bar"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("foo bar"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("foo bar barz"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optString: { not: { equals: "bar" }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optString: { not: { equals: "bar" }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn str_contains(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("bara"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("foo bar"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("foo bar barz"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optString: {contains: "bara" }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optString: { contains: "bara" }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn str_not_contains(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("bara"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("foo bar"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("foo bar barz"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optString: { not: { contains: "bara" }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optString: { not: { contains: "bara" }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn str_starts_with(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("bara"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("foo bar"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("foo bar barz"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optString: { startsWith: "bar" }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optString: { startsWith: "bar" }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn str_not_starts_with(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("bara"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("foo bar"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("foo bar barz"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optString: { not: { startsWith: "bar" }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optString: { not: { startsWith: "bar" }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn str_ends_with(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("bara"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("foo bar"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("foo bar bar"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optString: { endsWith: "bara" }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optString: { endsWith: "bara" }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn str_not_ends_with(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("bara"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("foo bar"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("foo bar bar"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;


        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optString: { not: { endsWith: "bara" }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optString: { not: { endsWith: "bara" }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn str_lt(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optString: { lt: "2" }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optString: { lt: "2" }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn str_lte(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optString: { lte: "2" }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"},{"idTest":"id2"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optString: { lte: "2" }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"},{"idTest":"id2"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn str_gt(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optString: { gt: "2" }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optString: { gt: "2" }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id3"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn str_gte(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optString: { gte: "2" }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optString: { gte: "2" }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn str_in(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("a"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("ab"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("abc"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optString: { in: ["a"] }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optString: { in: ["a"]}}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optString: { in: ["a","b"] }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optString: { in: ["a","b"] }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optString: { in: ["a","abc"] }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optString: { in: ["a","abc"] }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optString: { in: []}}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optString: { in: []}}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn str_not_in(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("a"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("ab"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("abc"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optString: { not: { in: ["a"] }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optString: { not: { in: ["a"] }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(orderBy: { idTest: asc }, where: {optString: { not: { in: [] }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"},{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(orderBy: { idTest: asc }, where: { b: { is: { int: { equals: 1 }}}, optString: { not: { in: [] }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"},{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        Ok(())
    }

    // Int tests
    #[rustfmt::skip]
    #[connector_test]
    async fn int_eq(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 2, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 3, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optInt: { equals: 1 }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optInt: { equals: 1 }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn int_not_eq(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("a"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("ab"), 2, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("abc"), 3, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optInt: { not: { equals: 1 }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optInt: { not: { equals: 1 }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn int_lt(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 2, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 3, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optInt: { lt: 2 }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optInt: { lt: 2 }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn int_lte(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 2, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 3, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;


        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optInt: { lte: 2 }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"},{"idTest":"id2"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optInt: { lte: 2 }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"},{"idTest":"id2"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn int_gt(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 2, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 3, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optInt: { gt: 2 }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optInt: { gt: 2 }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id3"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn int_gte(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 2, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 3, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optInt: { gte: 2 }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optInt: { gte: 2 }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn int_in(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("a"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("ab"), 2, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("abc"), 3, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optInt: { in: [1] }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optInt: { in: [1] }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn int_not_in(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("a"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("ab"), 2, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("abc"), 3, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optInt: { not: { in: [1] }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optInt: { not: { in: [1] }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        Ok(())
    }

    // Float tests
    #[rustfmt::skip]
    #[connector_test]
    async fn float_eq(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 2, 2.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 3, 3.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optFloat: { equals: 1.0 }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optFloat: { equals: 1.0 }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn float_not_eq(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("a"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("ab"), 2, 2.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("abc"), 3, 3.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optFloat: { not: { equals: 1.0 }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optFloat: { not: { equals: 1.0 }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn float_lt(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 2, 2.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 3, 3.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optFloat: { lt: 2.0 }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optFloat: { lt: 2.0 }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn float_lte(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 2, 2.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 3, 3.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optFloat: { lte: 2.0 }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"},{"idTest":"id2"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optFloat: { lte: 2.0 }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"},{"idTest":"id2"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn float_gt(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 2, 2.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 3, 3.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optFloat: { gt: 2.0 }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optFloat: { gt: 2.0 }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id3"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn float_gte(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 2, 2.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 3, 3.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optFloat: { gte: 2.0 }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optFloat: { gte: 2.0 }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn float_in(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("a"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("ab"), 2, 2.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("abc"), 3, 3.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optFloat: { in: [1.0] }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optFloat: { in: [1.0] }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn float_not_in(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("a"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("ab"), 2, 2.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("abc"), 3, 3.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optFloat: { not: { in: [1.0] }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optFloat: { not: { in: [1.0] }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        Ok(())
    }

    // Boolean tests
    #[rustfmt::skip]
    #[connector_test]
    async fn bool_eq(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("bar"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("foo bar"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("foo bar barz"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optBoolean: { equals: true }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optBoolean: { equals: true }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn bool_not_eq(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("bar"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("foo bar"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("foo bar barz"), 1, 1.0, false, "A", "2016-09-23T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: {optBoolean: { not: { equals: true }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optBoolean: { not: { equals: true }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        Ok(())
    }

    // DateTime tests
    #[rustfmt::skip]
    #[connector_test]
    async fn dt_eq(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 2, 2.0, false, "A", "2016-09-24T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 3, 3.0, false, "A", "2016-09-25T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optDateTime: { equals: "2016-09-24T12:29:32.342Z" }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optDateTime: { equals: "2016-09-24T12:29:32.342Z" }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn dt_not_eq(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 2, 2.0, false, "A", "2016-09-24T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 3, 3.0, false, "A", "2016-09-25T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optDateTime: { not: { equals: "2016-09-24T12:29:32.342Z" }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optDateTime: { not: { equals: "2016-09-24T12:29:32.342Z" }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"},{"idTest":"id3"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn dt_lt(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 2, 2.0, false, "A", "2016-09-24T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 3, 3.0, false, "A", "2016-09-25T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optDateTime: { lt: "2016-09-24T12:29:32.342Z" }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optDateTime: { lt: "2016-09-24T12:29:32.342Z" }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn dt_lte(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 2, 2.0, false, "A", "2016-09-24T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 3, 3.0, false, "A", "2016-09-25T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optDateTime: { lte: "2016-09-24T12:29:32.342Z" }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"},{"idTest":"id2"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optDateTime: { lte: "2016-09-24T12:29:32.342Z" }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"},{"idTest":"id2"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn dt_gt(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 2, 2.0, false, "A", "2016-09-24T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 3, 3.0, false, "A", "2016-09-25T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optDateTime: { gt: "2016-09-24T12:29:32.342Z" }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optDateTime: { gt: "2016-09-24T12:29:32.342Z" }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id3"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn dt_gte(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 2, 2.0, false, "A", "2016-09-24T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 3, 3.0, false, "A", "2016-09-25T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optDateTime: { gte: "2016-09-24T12:29:32.342Z" }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optDateTime: { gte: "2016-09-24T12:29:32.342Z" }}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        Ok(())
    }
    #[rustfmt::skip]
    #[connector_test]
    async fn dt_in(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 2, 2.0, false, "A", "2016-09-24T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 3, 3.0, false, "A", "2016-09-25T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optDateTime: { in: ["2016-09-24T12:29:32.342Z"] }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optDateTime: { in: ["2016-09-24T12:29:32.342Z"] }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn dt_not_in(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 2, 2.0, false, "A", "2016-09-24T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 3, 3.0, false, "A", "2016-09-25T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optDateTime: { not: { in: ["2016-09-24T12:29:32.342Z"] }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optDateTime: { not: { in: ["2016-09-24T12:29:32.342Z"] }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"},{"idTest":"id3"}]}}"###
        );

        Ok(())
    }

    // Enum tests
    #[rustfmt::skip]
    #[connector_test]
    async fn enum_eq(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 2, 2.0, false, "B", "2016-09-24T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 3, 3.0, false, "B", "2016-09-25T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optEnum: { equals: A }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optEnum: { equals: A }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn enum_not_eq(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 2, 2.0, false, "B", "2016-09-24T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 3, 3.0, false, "B", "2016-09-25T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optEnum: { not: { equals: A }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optEnum: { not: { equals: A }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn enum_in(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 2, 2.0, false, "B", "2016-09-24T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 3, 3.0, false, "B", "2016-09-25T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optEnum: { in: [A] }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optEnum: { in: [A] }}) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id1"}]}}"###
        );

        Ok(())
    }

    #[rustfmt::skip]
    #[connector_test]
    async fn enum_not_in(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 2, 2.0, false, "B", "2016-09-24T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 3, 3.0, false, "B", "2016-09-25T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optEnum: { not: { in: [A] }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { b: { is: { int: { equals: 1 }}}, optEnum: { not: { in: [A] }}}, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"},{"idTest":"id3"}]}}"###
        );

        Ok(())
    }

    // Misc tests
    #[rustfmt::skip]
    #[connector_test]
    async fn not_alias_equal(runner: Runner) -> TestResult<()> {
        create_common_model_b(&runner).await?;

        create_model_a(&runner, "id1", Some("1"), 1, 1.0, true, "A", "2016-09-23T12:29:32.342Z").await?;
        create_model_a(&runner, "id2", Some("2"), 2, 2.0, false, "B", "2016-09-24T12:29:32.342Z").await?;
        create_model_a(&runner, "id3", Some("3"), 3, 3.0, false, "B", "2016-09-25T12:29:32.342Z").await?;

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { optString: { not: { equals: "1", gt: "2" } } }, orderBy: { id: asc }) { idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{findManyModelA(where: { NOT: [ { optString: { equals: "1" }}, { optString: { gt: "2" }}  ]}, orderBy: { id: asc }) {idTest }}"#),
            @r###"{"data":{"findManyModelA":[{"idTest":"id2"}]}}"###
        );

        Ok(())
    }

    async fn create_model_a(
        runner: &Runner,
        id: &str,
        string: Option<&str>,
        int: usize,
        float: f64,
        boolean: bool,
        enum_: &str,
        datetime: &str,
    ) -> TestResult<()> {
        let string = match string {
            Some(string) => format!(r#""{}""#, string),
            None => String::from("null"),
        };

        // For some reson this doesn't resolve correctly.
        let query = format!(
            r#"
                mutation {{
                    createOneModelA(data: {{
                        idTest: "{}",
                        optString: {},
                        optInt: {},
                        optFloat: {},
                        optBoolean: {},
                        optEnum: {},
                        optDateTime: "{}"
                        b: {{ connect: {{ int: 1 }} }}
                    }}) {{ id }}
                }}
            "#,
            id, string, int, float, boolean, enum_, datetime
        );

        runner.query(query).await?.assert_success();

        Ok(())
    }

    async fn create_common_model_b(runner: &Runner) -> TestResult<()> {
        runner
            .query("mutation { createOneModelB(data: { int: 1 }) { id }}")
            .await?
            .assert_success();

        Ok(())
    }
}
