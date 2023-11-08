use query_engine_tests::*;

#[test_suite(schema(schema))]
mod max_integer {
    use query_engine_tests::Runner;

    fn schema() -> String {
        let schema = indoc! {r#"
        model Test {
            #id(id, Int, @id)
            int Int
        }
        "#};

        schema.to_string()
    }

    const I8_OVERFLOW_MAX: i64 = (i8::MAX as i64) + 1;
    const I8_OVERFLOW_MIN: i64 = (i8::MIN as i64) - 1;

    const I16_OVERFLOW_MAX: i64 = (i16::MAX as i64) + 1;
    const I16_OVERFLOW_MIN: i64 = (i16::MIN as i64) - 1;

    const I24_OVERFLOW_MAX: i64 = 8388607 + 1;
    const I24_OVERFLOW_MIN: i64 = -8388608 - 1;

    const I32_OVERFLOW_MAX: i64 = (i32::MAX as i64) + 1;
    const I32_OVERFLOW_MIN: i64 = (i32::MIN as i64) - 1;

    const U8_OVERFLOW_MAX: i64 = (u8::MAX as i64) + 1;
    const U16_OVERFLOW_MAX: i64 = (u16::MAX as i64) + 1;
    const U24_OVERFLOW_MAX: i64 = 16777215 + 1;
    const U32_OVERFLOW_MAX: i64 = (u32::MAX as i64) + 1;
    const OVERFLOW_MIN: i8 = -1;

    #[connector_test]
    async fn transform_gql_parser_too_large(runner: Runner) -> TestResult<()> {
        match runner.protocol() {
            query_engine_tests::EngineProtocol::Graphql => {
                assert_error!(
                    runner,
                    "mutation { createOneTest(data: { id: 1, int: 100000000000000000000 }) { id int } }",
                    2033,
                    "A number used in the query does not fit into a 64 bit signed integer. Consider using `BigInt` as field type if you're trying to store large integers."
                );
            }
            query_engine_tests::EngineProtocol::Json => {
                let res = runner
                    .query_json(
                        r#"{
                        "modelName": "Test",
                        "action": "createOne",
                        "query": {
                            "arguments": {
                                "data": {
                                    "id": 1,
                                    "int": 100000000000000000000
                                }
                            },
                            "selection": {
                                "id": true,
                                "int": true
                            }
                        }
                    }"#,
                    )
                    .await?;

                res.assert_failure(2009, Some("Unable to fit float value (or large JS integer serialized in exponent notation) '100000000000000000000' into a 64 Bit signed integer for field 'int'".to_string()))
            }
        }

        Ok(())
    }

    #[connector_test]
    async fn transform_gql_parser_too_small(runner: Runner) -> TestResult<()> {
        match runner.protocol() {
            query_engine_tests::EngineProtocol::Graphql => {
                assert_error!(
                    runner,
                    "mutation { createOneTest(data: { id: 1, int: -100000000000000000000 }) { id int } }",
                    2033,
                    "A number used in the query does not fit into a 64 bit signed integer. Consider using `BigInt` as field type if you're trying to store large integers."
                );
            }
            query_engine_tests::EngineProtocol::Json => {
                let res = runner
                    .query_json(
                        r#"{
                        "modelName": "Test",
                        "action": "createOne",
                        "query": {
                            "arguments": {
                                "data": {
                                    "id": 1,
                                    "int": -100000000000000000000
                                }
                            },
                            "selection": {
                                "id": true,
                                "int": true
                            }
                        }
                    }"#,
                    )
                    .await?;

                res.assert_failure(2009, Some("Unable to fit float value (or large JS integer serialized in exponent notation) '-100000000000000000000' into a 64 Bit signed integer for field 'int'".to_string()))
            }
        }

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

    // All connectors error differently based on their database driver.
    // We just assert that a basic overflowing int errors without checking specifically for the message.
    // Specific messages are asserted down below for native types.
    // MongoDB is excluded because it automatically upcasts a value as an i64 if doesn't fit in an i32.
    // MySQL 5.6 is excluded because it never overflows but inserts the min or max of the range of the column type instead.
    #[connector_test(exclude(MongoDb, MySql(5.6)))]
    async fn unfitted_int_should_fail(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ id: 1, int: {I32_OVERFLOW_MAX} }}) {{ id int }} }}"),
            0
        );

        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ id: 1, int: {I32_OVERFLOW_MIN} }}) {{ id int }} }}"),
            0
        );

        Ok(())
    }

    fn overflow_pg() -> String {
        let schema = indoc! {
            r#"model Test {
                id Int @id @default(autoincrement())
                int Int? @test.Integer
                smallint Int? @test.SmallInt
                oid Int? @test.Oid
            }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(overflow_pg), only(Postgres), exclude(JS))]
    async fn unfitted_int_should_fail_pg_quaint(runner: Runner) -> TestResult<()> {
        // int
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ int: {I32_OVERFLOW_MAX} }}) {{ id }} }}"),
            None,
            "Unable to fit integer value '2147483648' into an INT4 (32-bit signed integer)."
        );
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ int: {I32_OVERFLOW_MIN} }}) {{ id }} }}"),
            None,
            "Unable to fit integer value '-2147483649' into an INT4 (32-bit signed integer)."
        );

        // smallint
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ smallint: {I16_OVERFLOW_MAX} }}) {{ id }} }}"),
            None,
            "Unable to fit integer value '32768' into an INT2 (16-bit signed integer)."
        );
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ smallint: {I16_OVERFLOW_MIN} }}) {{ id }} }}"),
            None,
            "Unable to fit integer value '-32769' into an INT2 (16-bit signed integer)."
        );

        //oid
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ oid: {U32_OVERFLOW_MAX} }}) {{ id }} }}"),
            None,
            "Unable to fit integer value '4294967296' into an OID (32-bit unsigned integer)."
        );
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ oid: {OVERFLOW_MIN} }}) {{ id }} }}"),
            None,
            "Unable to fit integer value '-1' into an OID (32-bit unsigned integer)."
        );

        Ok(())
    }

    // The driver adapter for neon provides different error messages on overflow
    #[connector_test(schema(overflow_pg), only(JS, Postgres))]
    async fn unfitted_int_should_fail_pg_js(runner: Runner) -> TestResult<()> {
        // int
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ int: {I32_OVERFLOW_MAX} }}) {{ id }} }}"),
            None,
            "value \\\"2147483648\\\" is out of range for type integer"
        );
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ int: {I32_OVERFLOW_MIN} }}) {{ id }} }}"),
            None,
            "value \\\"-2147483649\\\" is out of range for type integer"
        );

        // smallint
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ smallint: {I16_OVERFLOW_MAX} }}) {{ id }} }}"),
            None,
            "value \\\"32768\\\" is out of range for type smallint"
        );
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ smallint: {I16_OVERFLOW_MIN} }}) {{ id }} }}"),
            None,
            "value \\\"-32769\\\" is out of range for type smallint"
        );

        //oid
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ oid: {U32_OVERFLOW_MAX} }}) {{ id }} }}"),
            None,
            "value \\\"4294967296\\\" is out of range for type oid"
        );

        // The underlying driver swallows a negative id by interpreting it as unsigned.
        // {"data":{"createOneTest":{"id":1,"oid":4294967295}}}
        run_query!(
            runner,
            format!("mutation {{ createOneTest(data: {{ oid: {OVERFLOW_MIN} }}) {{ id, oid }} }}")
        );

        Ok(())
    }

    #[connector_test(schema(overflow_pg), only(Postgres))]
    async fn fitted_int_should_work_pg(runner: Runner) -> TestResult<()> {
        // int
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ int: {} }}) {{ id int }} }}", i32::MAX)),
          @r###"{"data":{"createOneTest":{"id":1,"int":2147483647}}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ int: {} }}) {{ id int }} }}", i32::MIN)),
          @r###"{"data":{"createOneTest":{"id":2,"int":-2147483648}}}"###
        );

        // smallint
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ smallint: {} }}) {{ id smallint }} }}", i16::MAX)),
          @r###"{"data":{"createOneTest":{"id":3,"smallint":32767}}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ smallint: {} }}) {{ id smallint }} }}", i16::MIN)),
          @r###"{"data":{"createOneTest":{"id":4,"smallint":-32768}}}"###
        );

        // oid
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ oid: {} }}) {{ id oid }} }}", u32::MAX)),
          @r###"{"data":{"createOneTest":{"id":5,"oid":4294967295}}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ oid: {} }}) {{ id oid }} }}", u32::MIN)),
          @r###"{"data":{"createOneTest":{"id":6,"oid":0}}}"###
        );

        Ok(())
    }

    fn overflow_mysql() -> String {
        let schema = indoc! {
            r#"model Test {
                id        Int @id @default(autoincrement())
                tinyint   Int? @test.TinyInt
                smallint  Int? @test.SmallInt
                mediumint Int? @test.MediumInt
                int       Int? @test.Int
                year      Int? @test.Year

                unsigned_tinyint   Int? @test.UnsignedTinyInt
                unsigned_smallint  Int? @test.UnsignedSmallInt
                unsigned_mediumint Int? @test.UnsignedMediumInt
                unsigned_int       Int? @test.UnsignedInt
            }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(overflow_mysql), only(MySql(5.7, 8, "mariadb")))]
    async fn unfitted_int_should_fail_mysql(runner: Runner) -> TestResult<()> {
        // tinyint
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ tinyint: {I8_OVERFLOW_MAX} }}) {{ id }} }}"),
            2020,
            "Value out of range for the type. Out of range value for column 'tinyint'"
        );
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ tinyint: {I8_OVERFLOW_MIN} }}) {{ id }} }}"),
            2020,
            "Value out of range for the type. Out of range value for column 'tinyint'"
        );

        // smallint
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ smallint: {I16_OVERFLOW_MAX} }}) {{ id }} }}"),
            2020,
            "Value out of range for the type. Out of range value for column 'smallint'"
        );
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ smallint: {I16_OVERFLOW_MIN} }}) {{ id }} }}"),
            2020,
            "Value out of range for the type. Out of range value for column 'smallint'"
        );

        // mediumint
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ mediumint: {I24_OVERFLOW_MAX} }}) {{ id }} }}"),
            2020,
            "Value out of range for the type. Out of range value for column 'mediumint'"
        );
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ mediumint: {I24_OVERFLOW_MIN} }}) {{ id }} }}"),
            2020,
            "Value out of range for the type. Out of range value for column 'mediumint'"
        );

        // int
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ int: {I32_OVERFLOW_MAX} }}) {{ id }} }}"),
            2020,
            "Value out of range for the type. Out of range value for column 'int'"
        );
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ int: {I32_OVERFLOW_MIN} }}) {{ id }} }}"),
            2020,
            "Value out of range for the type. Out of range value for column 'int'"
        );

        // year
        // Type year is stored as an 8-bit unsigned integer but the actual boundaries are 1901-2155.
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ year: {} }}) {{ id }} }}", 2156),
            2020,
            "Value out of range for the type. Out of range value for column 'year'"
        );
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ year: {} }}) {{ id }} }}", 1900),
            2020,
            "Value out of range for the type. Out of range value for column 'year'"
        );

        // unsigned tinyint
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ unsigned_tinyint: {U8_OVERFLOW_MAX} }}) {{ id }} }}"),
            2020,
            "Value out of range for the type. Out of range value for column 'unsigned_tinyint'"
        );
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ unsigned_tinyint: {OVERFLOW_MIN} }}) {{ id }} }}"),
            2020,
            "Value out of range for the type. Out of range value for column 'unsigned_tinyint'"
        );

        // unsigned smallint
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ unsigned_smallint: {U16_OVERFLOW_MAX} }}) {{ id }} }}"),
            2020,
            "Value out of range for the type. Out of range value for column 'unsigned_smallint'"
        );
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ unsigned_smallint: {OVERFLOW_MIN} }}) {{ id }} }}"),
            2020,
            "Value out of range for the type. Out of range value for column 'unsigned_smallint'"
        );

        // unsigned mediumint
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ unsigned_mediumint: {U24_OVERFLOW_MAX} }}) {{ id }} }}"),
            2020,
            "Value out of range for the type. Out of range value for column 'unsigned_mediumint'"
        );
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ unsigned_mediumint: {OVERFLOW_MIN} }}) {{ id }} }}"),
            2020,
            "Value out of range for the type. Out of range value for column 'unsigned_mediumint'"
        );

        // unsigned int
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ unsigned_int: {U32_OVERFLOW_MAX} }}) {{ id }} }}"),
            2020,
            "Value out of range for the type. Out of range value for column 'unsigned_int'"
        );
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ unsigned_int: {OVERFLOW_MIN} }}) {{ id }} }}"),
            2020,
            "Value out of range for the type. Out of range value for column 'unsigned_int'"
        );

        Ok(())
    }

    #[connector_test(schema(overflow_mysql), only(MySql))]
    async fn fitted_int_should_work_mysql(runner: Runner) -> TestResult<()> {
        // tinyint
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ tinyint: {} }}) {{ tinyint }} }}", i8::MAX)),
          @r###"{"data":{"createOneTest":{"tinyint":127}}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ tinyint: {} }}) {{ tinyint }} }}", i8::MIN)),
          @r###"{"data":{"createOneTest":{"tinyint":-128}}}"###
        );

        // smallint
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ smallint: {} }}) {{ smallint }} }}", i16::MAX)),
          @r###"{"data":{"createOneTest":{"smallint":32767}}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ smallint: {} }}) {{ smallint }} }}", i16::MIN)),
          @r###"{"data":{"createOneTest":{"smallint":-32768}}}"###
        );

        // mediumint
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ mediumint: {} }}) {{ mediumint }} }}", (I24_OVERFLOW_MAX - 1))),
          @r###"{"data":{"createOneTest":{"mediumint":8388607}}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ mediumint: {} }}) {{ mediumint }} }}", (I24_OVERFLOW_MIN + 1))),
          @r###"{"data":{"createOneTest":{"mediumint":-8388608}}}"###
        );

        // int
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ int: {} }}) {{ int }} }}", i32::MAX)),
          @r###"{"data":{"createOneTest":{"int":2147483647}}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ int: {} }}) {{ int }} }}", i32::MIN)),
          @r###"{"data":{"createOneTest":{"int":-2147483648}}}"###
        );

        // year
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ year: {} }}) {{ year }} }}", 2155)),
          @r###"{"data":{"createOneTest":{"year":2155}}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ year: {} }}) {{ year }} }}", 1901)),
          @r###"{"data":{"createOneTest":{"year":1901}}}"###
        );

        // unsigned_tinyint
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ unsigned_tinyint: {} }}) {{ unsigned_tinyint }} }}", u8::MAX)),
          @r###"{"data":{"createOneTest":{"unsigned_tinyint":255}}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ unsigned_tinyint: {} }}) {{ unsigned_tinyint }} }}", u8::MIN)),
          @r###"{"data":{"createOneTest":{"unsigned_tinyint":0}}}"###
        );

        // unsigned_smallint
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ unsigned_smallint: {} }}) {{ unsigned_smallint }} }}", u16::MAX)),
          @r###"{"data":{"createOneTest":{"unsigned_smallint":65535}}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ unsigned_smallint: {} }}) {{ unsigned_smallint }} }}", u16::MIN)),
          @r###"{"data":{"createOneTest":{"unsigned_smallint":0}}}"###
        );

        // unsigned_mediumint
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ unsigned_mediumint: {} }}) {{ unsigned_mediumint }} }}", U24_OVERFLOW_MAX - 1)),
          @r###"{"data":{"createOneTest":{"unsigned_mediumint":16777215}}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ unsigned_mediumint: {} }}) {{ unsigned_mediumint }} }}", OVERFLOW_MIN + 1)),
          @r###"{"data":{"createOneTest":{"unsigned_mediumint":0}}}"###
        );

        // unsigned int
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ unsigned_int: {} }}) {{ unsigned_int }} }}", u32::MAX)),
          @r###"{"data":{"createOneTest":{"unsigned_int":4294967295}}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ unsigned_int: {} }}) {{ unsigned_int }} }}", u32::MIN)),
          @r###"{"data":{"createOneTest":{"unsigned_int":0}}}"###
        );

        Ok(())
    }

    fn overflow_mssql() -> String {
        let schema = indoc! {
            r#"model Test {
                id Int @id @default(autoincrement())
                tinyint Int? @test.TinyInt
                smallint Int? @test.SmallInt
                int Int? @test.Int
            }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(overflow_mssql), only(SqlServer))]
    async fn unfitted_int_should_fail_mssql(runner: Runner) -> TestResult<()> {
        // tinyint
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ tinyint: {U8_OVERFLOW_MAX} }}) {{ id }} }}"),
            None,
            "Arithmetic overflow error converting expression to data type tinyint"
        );
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ tinyint: {OVERFLOW_MIN} }}) {{ id }} }}"),
            None,
            "Arithmetic overflow error converting expression to data type tinyint"
        );

        // smallint
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ smallint: {I16_OVERFLOW_MAX} }}) {{ id }} }}"),
            None,
            "Arithmetic overflow error converting expression to data type smallint"
        );
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ smallint: {I16_OVERFLOW_MIN} }}) {{ id }} }}"),
            None,
            "Arithmetic overflow error converting expression to data type smallint."
        );

        // int
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ int: {I32_OVERFLOW_MAX} }}) {{ id }} }}"),
            None,
            "Arithmetic overflow error converting expression to data type int"
        );
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ int: {I32_OVERFLOW_MIN} }}) {{ id }} }}"),
            None,
            "Arithmetic overflow error converting expression to data type int"
        );

        Ok(())
    }

    #[connector_test(schema(overflow_mssql), only(SqlServer))]
    async fn fitted_int_should_work_mssql(runner: Runner) -> TestResult<()> {
        // tinyint
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ tinyint: {} }}) {{ tinyint }} }}", u8::MAX)),
          @r###"{"data":{"createOneTest":{"tinyint":255}}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ tinyint: {} }}) {{ tinyint }} }}", u8::MIN)),
          @r###"{"data":{"createOneTest":{"tinyint":0}}}"###
        );

        // smallint
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ smallint: {} }}) {{ smallint }} }}", i16::MAX)),
          @r###"{"data":{"createOneTest":{"smallint":32767}}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ smallint: {} }}) {{ smallint }} }}", i16::MIN)),
          @r###"{"data":{"createOneTest":{"smallint":-32768}}}"###
        );

        // int
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ int: {} }}) {{ int }} }}", i32::MAX)),
          @r###"{"data":{"createOneTest":{"int":2147483647}}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ int: {} }}) {{ int }} }}", i32::MIN)),
          @r###"{"data":{"createOneTest":{"int":-2147483648}}}"###
        );

        Ok(())
    }

    fn overflow_cockroach() -> String {
        let schema = indoc! {
            r#"model Test {
                id Int @id
                int2 Int? @test.Int2
                int4 Int? @test.Int4
                oid  Int? @test.Oid
            }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(overflow_cockroach), only(CockroachDb))]
    async fn unfitted_int_should_fail_cockroach(runner: Runner) -> TestResult<()> {
        // int4
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ id: 1, int4: {I32_OVERFLOW_MAX} }}) {{ id }} }}"),
            None,
            "Unable to fit integer value '2147483648' into an INT4 (32-bit signed integer)."
        );
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ id: 1, int4: {I32_OVERFLOW_MIN} }}) {{ id }} }}"),
            None,
            "Unable to fit integer value '-2147483649' into an INT4 (32-bit signed integer)."
        );

        // int2
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ id: 1, int2: {I16_OVERFLOW_MAX} }}) {{ id }} }}"),
            None,
            "Unable to fit integer value '32768' into an INT2 (16-bit signed integer)."
        );
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ id: 1, int2: {I16_OVERFLOW_MIN} }}) {{ id }} }}"),
            None,
            "Unable to fit integer value '-32769' into an INT2 (16-bit signed integer)."
        );

        //oid
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ id: 1, oid: {U32_OVERFLOW_MAX} }}) {{ id }} }}"),
            None,
            "Unable to fit integer value '4294967296' into an OID (32-bit unsigned integer)."
        );
        assert_error!(
            runner,
            format!("mutation {{ createOneTest(data: {{ id: 1, oid: {OVERFLOW_MIN} }}) {{ id }} }}"),
            None,
            "Unable to fit integer value '-1' into an OID (32-bit unsigned integer)."
        );

        Ok(())
    }

    #[connector_test(schema(overflow_cockroach), only(CockroachDb))]
    async fn fitted_int_should_work_cockroach(runner: Runner) -> TestResult<()> {
        // int2
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ id: 1, int2: {} }}) {{ id int2 }} }}", i16::MAX)),
          @r###"{"data":{"createOneTest":{"id":1,"int2":32767}}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ id: 2, int2: {} }}) {{ id int2 }} }}", i16::MIN)),
          @r###"{"data":{"createOneTest":{"id":2,"int2":-32768}}}"###
        );

        // int4
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ id: 3, int4: {} }}) {{ id int4 }} }}", i32::MAX)),
          @r###"{"data":{"createOneTest":{"id":3,"int4":2147483647}}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ id: 4, int4: {} }}) {{ id int4 }} }}", i32::MIN)),
          @r###"{"data":{"createOneTest":{"id":4,"int4":-2147483648}}}"###
        );

        // oid
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ id: 5, oid: {} }}) {{ id oid }} }}", u32::MAX)),
          @r###"{"data":{"createOneTest":{"id":5,"oid":4294967295}}}"###
        );
        insta::assert_snapshot!(
          run_query!(&runner, format!("mutation {{ createOneTest(data: {{ id: 6, oid: {} }}) {{ id oid }} }}", u32::MIN)),
          @r###"{"data":{"createOneTest":{"id":6,"oid":0}}}"###
        );

        Ok(())
    }
}

#[test_suite(schema(schema))]
mod float_serialization_issues {
    use query_engine_tests::Runner;

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
        match runner.protocol() {
            query_engine_tests::EngineProtocol::Graphql => {
                assert_error!(
                    runner,
                    "mutation { createOneTest(data: { id: 1, float: 100000000000000000000 }) { id float } }",
                    2033,
                    "A number used in the query does not fit into a 64 bit signed integer. Consider using `BigInt` as field type if you're trying to store large integers."
                );
            }
            query_engine_tests::EngineProtocol::Json => {
                let res = runner
                    .query_json(
                        r#"{
                        "modelName": "Test",
                        "action": "createOne",
                        "query": {
                            "arguments": {
                                "data": {
                                    "id": 1,
                                    "float": 100000000000000000000
                                }
                            },
                            "selection": {
                                "id": true,
                                "float": true
                            }
                        }
                    }"#,
                    )
                    .await?;

                // Succeeds because the JSON protocol lifts some limitation of the GraphQL parser.
                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"createOneTest":{"id":1,"float":1e20}}}"###
                );
            }
        }

        Ok(())
    }
}
