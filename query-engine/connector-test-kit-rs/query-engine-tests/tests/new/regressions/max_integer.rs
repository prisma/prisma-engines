use query_engine_tests::*;

#[test_suite(schema(schema))]
mod max_integer {
    fn schema() -> String {
        let schema = indoc! {r#"
        model Test {
            #id(id, Int, @id)
            int Int
        }
        "#};

        schema.to_string()
    }

    #[connector_test]
    async fn transform_gql_parser_too_large(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            "mutation { createOneTest(data: { id: 1, int: 100000000000000000000 }) { id int } }",
            2033,
            "A number used in the query does not fit into a 64 bit signed integer. Consider using `BigInt` as field type if you're trying to store large integers."
        );

        Ok(())
    }

    #[connector_test]
    async fn transform_gql_parser_too_small(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            "mutation { createOneTest(data: { id: 1, int: -100000000000000000000 }) { id int } }",
            2033,
            "A number used in the query does not fit into a 64 bit signed integer. Consider using `BigInt` as field type if you're trying to store large integers."
        );

        Ok(())
    }

    // The document parser does not crash on encountering an exponent-notation-serialized int.
    // This triggers a 2009 instead of 2033 as this is in the document parser.
    #[connector_test]
    async fn document_parser_no_crash_too_large(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            "mutation { createOneTest(data: { id: 1, int: 1e20 }) { id int } }",
            2009,
            "Unable to fit float value (or large JS integer serialized in exponent notation) '100000000000000000000' into a 64 Bit signed integer for field 'int'. If you're trying to store large integers, consider using `BigInt`"
        );

        Ok(())
    }

    #[connector_test]
    async fn document_parser_no_crash_too_small(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            "mutation { createOneTest(data: { id: 1, int: -1e20 }) { id int } }",
            2009,
            "Unable to fit float value (or large JS integer serialized in exponent notation) '-100000000000000000000' into a 64 Bit signed integer for field 'int'. If you're trying to store large integers, consider using `BigInt`"
        );

        Ok(())
    }

    // This will not work anymore in the future as we'll redo the float / decimal story. Right now this "works" because floats are deserialized as BigDecimal.
    #[connector_test]
    async fn document_parser_no_crash_ridiculously_big(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            "mutation { createOneTest(data: { id: 1, int: 1e100 }) { id int } }",
            2009,
            "Unable to fit float value (or large JS integer serialized in exponent notation) '10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000' into a 64 Bit signed integer for field 'int'. If you're trying to store large integers, consider using `BigInt`"
        );

        Ok(())
    }
}

#[test_suite(schema(schema))]
mod float_serialization_issues {
    fn schema() -> String {
        let schema = indoc! {r#"
        model Test {
            #id(id, Int, @id)
            float Float
        }
        "#};

        schema.to_string()
    }

    #[connector_test(exclude(SqlServer))]
    async fn int_range_overlap_works(runner: Runner) -> TestResult<()> {
        runner
            .query("mutation { createOneTest(data: { id: 1, float: 1e20 }) { id float } }")
            .await?
            .assert_success();

        Ok(())
    }

    // The same number as above, just not in the exponent notation. That one fails, because f64 can represent the number, i64 can't.
    #[connector_test]
    async fn int_range_overlap_fails(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            "mutation { createOneTest(data: { id: 1, float: 100000000000000000000 }) { id float } }",
            2033,
            "A number used in the query does not fit into a 64 bit signed integer. Consider using `BigInt` as field type if you're trying to store large integers."
        );

        Ok(())
    }
}
