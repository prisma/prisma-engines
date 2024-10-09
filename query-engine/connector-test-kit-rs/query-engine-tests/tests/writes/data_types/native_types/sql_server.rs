use query_engine_tests::*;

#[test_suite(only(SqlServer))]
mod sql_server {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema_int_int() -> String {
        let schema = indoc! {
            r#"model Model {
              #id(inc_int, Int, @id, @default(autoincrement()) @test.Int)
              int  Int     @test.Int
              sInt Int     @test.SmallInt
              tInt Int     @test.TinyInt
              bInt BigInt  @test.BigInt
              bit  Int     @test.Bit
              bool Boolean @test.Bit
            }"#
        };

        schema.to_owned()
    }

    fn schema_int_tinyint() -> String {
        let schema = indoc! {
            r#"model Model {
            #id(inc_tInt, Int, @id, @default(autoincrement()) @test.TinyInt)
            int  Int     @test.Int
            sInt Int     @test.SmallInt
            tInt Int     @test.TinyInt
            bInt BigInt  @test.BigInt
            bit  Int     @test.Bit
            bool Boolean @test.Bit
          }"#
        };

        schema.to_owned()
    }

    fn schema_int_smallint() -> String {
        let schema = indoc! {
            r#"model Model {
            #id(inc_sInt, Int, @id, @default(autoincrement()) @test.SmallInt)
            int  Int     @test.Int
            sInt Int     @test.SmallInt
            tInt Int     @test.TinyInt
            bInt BigInt  @test.BigInt
            bit  Int     @test.Bit
            bool Boolean @test.Bit
          }"#
        };

        schema.to_owned()
    }

    fn schema_bigint_bigint() -> String {
        let schema = indoc! {
            r#"model Model {
          #id(inc_bInt, BigInt, @id, @default(autoincrement()) @test.BigInt)
          int  Int     @test.Int
          sInt Int     @test.SmallInt
          tInt Int     @test.TinyInt
          bInt BigInt  @test.BigInt
          bit  Int     @test.Bit
          bool Boolean @test.Bit
        }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_int_int))]
    async fn native_int_int(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                int: 2147483647
                sInt: 32767
                tInt: 12
                bInt: 5294967295
                bit: 1
                bool: false
              }
            ) {
              int
              sInt
              tInt
              bInt
              bit
              bool
              inc_int
            }
          }"#),
          @r###"{"data":{"createOneModel":{"int":2147483647,"sInt":32767,"tInt":12,"bInt":"5294967295","bit":1,"bool":false,"inc_int":1}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(schema_int_tinyint))]
    async fn native_int_tinyint(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
          createOneModel(
            data: {
              int: 2147483647
              sInt: 32767
              tInt: 12
              bInt: 5294967295
              bit: 1
              bool: false
            }
          ) {
            int
            sInt
            tInt
            bInt
            bit
            bool
            inc_tInt
          }
        }"#),
          @r###"{"data":{"createOneModel":{"int":2147483647,"sInt":32767,"tInt":12,"bInt":"5294967295","bit":1,"bool":false,"inc_tInt":1}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(schema_int_smallint))]
    async fn native_int_smallint(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
          createOneModel(
            data: {
              int: 2147483647
              sInt: 32767
              tInt: 12
              bInt: 5294967295
              bit: 1
              bool: false
            }
          ) {
            int
            sInt
            tInt
            bInt
            bit
            bool
            inc_sInt
          }
        }"#),
          @r###"{"data":{"createOneModel":{"int":2147483647,"sInt":32767,"tInt":12,"bInt":"5294967295","bit":1,"bool":false,"inc_sInt":1}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(schema_bigint_bigint))]
    async fn native_bigint_bigint(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
          createOneModel(
            data: {
              int: 2147483647
              sInt: 32767
              tInt: 12
              bInt: 5294967295
              bit: 1
              bool: false
            }
          ) {
            int
            sInt
            tInt
            bInt
            bit
            bool
            inc_bInt
          }
        }"#),
          @r###"{"data":{"createOneModel":{"int":2147483647,"sInt":32767,"tInt":12,"bInt":"5294967295","bit":1,"bool":false,"inc_bInt":"1"}}}"###
        );

        Ok(())
    }

    fn schema_decimal() -> String {
        let schema = indoc! {
            r#"model Model {
              #id(id, String, @id, @default(uuid()) @test.UniqueIdentifier)
              float      Float   @test.Real
              dfloat     Float   @test.Float(53)
              money      Float   @test.Money
              smallMoney Float   @test.SmallMoney
              decFloat   Decimal @test.Decimal(2, 1)
              decFloat2  Decimal @test.Decimal(10, 6)
            }"#
        };

        schema.to_owned()
    }

    // "SQL Server native decimal types" should "work"
    #[connector_test(schema(schema_decimal))]
    async fn native_decimal_type(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                float: 1.1
                dfloat: 2.2
                money: 22.14
                smallMoney: 22.12
                decFloat: 3.1234
                decFloat2: "4.12345"
              }
            ) {
              float
              dfloat
              money
              smallMoney
              decFloat
              decFloat2
            }
          }"#),
          // decFloat is cut due to precision
          @r###"{"data":{"createOneModel":{"float":1.1,"dfloat":2.2,"money":22.14,"smallMoney":22.12,"decFloat":"3.1","decFloat2":"4.12345"}}}"###
        );

        Ok(())
    }

    fn schema_string() -> String {
        let schema = indoc! {
            r#"model Model {
              #id(id, String, @id, @default(uuid()) @test.UniqueIdentifier)
              char   String @test.Char(10)
              nchar  String @test.NChar(10)
              vChar  String @test.VarChar(Max)
              nVChar String @test.NVarChar(1000)
              text   String @test.Text
              nText  String @test.NText
            }"#
        };

        schema.to_owned()
    }

    // "SQL Server native string types" should "work"
    #[connector_test(schema(schema_string))]
    async fn native_string_types(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                char: "1234567890"
                nchar: "1234567890"
                vChar: "12345678910"
                nVChar: "教育漢字教育漢字"
                text: "text"
                nText: "教育漢字"
              }
            ) {
              char
              nchar
              vChar
              nVChar
              text
              nText
            }
          }"#),
          @r###"{"data":{"createOneModel":{"char":"1234567890","nchar":"1234567890","vChar":"12345678910","nVChar":"教育漢字教育漢字","text":"text","nText":"教育漢字"}}}"###
        );

        Ok(())
    }

    fn schema_date_types() -> String {
        let schema = indoc! {
            r#"model Model {
              #id(id, String, @id, @default(uuid()) @test.UniqueIdentifier)
              date   DateTime @test.Date
              time   DateTime @test.Time
              dtime  DateTime @test.DateTime
              dtime2 DateTime @test.DateTime2
              dtoff  DateTime @test.DateTimeOffset
              small  DateTime @test.SmallDateTime
            }"#
        };

        schema.to_owned()
    }

    // "SQL Server native date types" should "work"
    #[connector_test(schema(schema_date_types))]
    async fn native_date_types(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                date: "2016-09-24T00:00:00.000Z"
                time: "2016-09-24T13:14:15.123Z"
                dtime: "2016-09-24T12:29:32.342Z"
                dtime2: "2016-09-24T12:29:32.342Z"
                dtoff: "2016-09-24T12:29:32.342Z"
                small: "2016-09-24T12:29:32.342Z"
              }
            ) {
              date
              time
              dtime
              dtime2
              dtoff
              small
            }
          }"#),
          @r###"{"data":{"createOneModel":{"date":"2016-09-24T00:00:00.000Z","time":"1970-01-01T13:14:15.123Z","dtime":"2016-09-24T12:29:32.343Z","dtime2":"2016-09-24T12:29:32.342Z","dtoff":"2016-09-24T12:29:32.342Z","small":"2016-09-24T12:30:00.000Z"}}}"###
        );

        Ok(())
    }

    fn schema_binary() -> String {
        let schema = indoc! {
            r#"model Model {
              #id(id, String, @id, @default(uuid()) @test.UniqueIdentifier)
              bin   Bytes @test.Binary(4)
              vBin  Bytes @test.VarBinary(Max)
              image Bytes @test.Image
            }"#
        };

        schema.to_owned()
    }

    // "SQL Server native binary types" should "work"
    #[connector_test(schema(schema_binary))]
    async fn native_binary_types(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                bin: "dGVzdA=="
                vBin: "dGVzdA=="
                image: "dGVzdA=="
              }
            ) {
              bin
              vBin
              image
            }
          }"#),
          @r###"{"data":{"createOneModel":{"bin":"dGVzdA==","vBin":"dGVzdA==","image":"dGVzdA=="}}}"###
        );

        Ok(())
    }

    fn schema_other_native_types() -> String {
        let schema = indoc! {
            r#"model Model {
              #id(id, String, @id, @default(cuid()))
              xml  String   @test.Xml
              uuid String   @test.UniqueIdentifier
              geom Geometry @test.Geometry
              geog Geometry @test.Geography
            }"#
        };

        schema.to_owned()
    }

    // "Other SQL Server native types" should "work"
    #[connector_test(schema(schema_other_native_types))]
    async fn other_native_types(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                xml: "<meow>purr</meow>"
                uuid: "ab309dfd-d041-4110-b162-75d7b95fe989"
                geom: "{\"type\": \"Point\", \"coordinates\": [1,2]}"
                geog: "{\"type\": \"Point\", \"coordinates\": [1,2]}"
              }
            ) {
              xml
              uuid
              geom
              geog
            }
          }"#),
          @r###"{"data":{"createOneModel":{"xml":"<meow>purr</meow>","uuid":"ab309dfd-d041-4110-b162-75d7b95fe989","geom":"{\"type\":\"Point\",\"coordinates\":[1,2]}","geog":"{\"type\":\"Point\",\"coordinates\":[1,2]}"}}}"###
        );

        Ok(())
    }

    fn schema_fixed_size_char_native_types() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, String, @id, @test.Char(16))
              b_id String? @test.Char(16) @unique
              b    ModelB? @relation(fields: [b_id], references: [id])
            }

            model ModelB {
              #id(id, String, @id, @test.Char(16))
              a  ModelA?
            }"#
        };

        schema.to_owned()
    }

    // "Sql server native fixed-size char type" should "be handled correctly wrt. padding for comparisons"
    #[connector_test(schema(schema_fixed_size_char_native_types))]
    async fn fixed_size_char_native_type(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModelA(data: {
              id: "1234"
               b: { create: { id: "4321" } }
            }) {
              id
              b { id }
            }
          }"#),
          // This is correct - sql server returns padded strings (as opposed to MySQL for example, where it's trimmed).
          @r###"{"data":{"createOneModelA":{"id":"1234            ","b":{"id":"4321            "}}}}"###
        );

        Ok(())
    }

    fn schema_fixed_size_n_char() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, String, @id, @test.NChar(16))
              b_id String? @test.NChar(16) @unique
              b    ModelB? @relation(fields: [b_id], references: [id])
            }

            model ModelB {
              #id(id, String, @id, @test.NChar(16))
              a  ModelA?
            }"#
        };

        schema.to_owned()
    }

    // "Sql server native fixed-size nchar type" should "be handled correctly wrt. padding for comparisons"
    #[connector_test(schema(schema_fixed_size_n_char))]
    async fn fixed_size_n_char_native_type(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModelA(data: {
              id: "1234"
               b: { create: { id: "4321" } }
            }) {
              id
              b { id }
            }
          }"#),
          @r###"{"data":{"createOneModelA":{"id":"1234            ","b":{"id":"4321            "}}}}"###
        );

        Ok(())
    }
}
