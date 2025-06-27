use indoc::indoc;
use query_engine_tests::*;

#[test_suite(only(Postgres, CockroachDb))]
mod postgres_datetime {
    fn schema_date() -> String {
        let schema = indoc! {
            r#"model Parent {
              #id(id, Int, @id)

              childId Int? @unique
              child Child? @relation(fields: [childId], references: [id])
            }
            
            model Child {
                #id(id, Int, @id)
                date       DateTime @test.Date
                date_2     DateTime @test.Date
                time       DateTime @test.Time(3)
                time_2     DateTime @test.Time(3)
                time_tz    DateTime @test.Timetz(3)
                time_tz_2  DateTime @test.Timetz(3)
                ts         DateTime @test.Timestamp(3)
                ts_2       DateTime @test.Timestamp(3)
                ts_tz      DateTime @test.Timestamptz(3)
                ts_tz_2    DateTime @test.Timestamptz(3)

                parent Parent?
            }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_date))]
    async fn dt_native(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
            id: 1,
            child: { create: {
                id: 1,
                date: "2016-09-24T00:00:00.000Z"
                date_2: "2016-09-24T00:00:00.000+03:00"
                time: "1111-11-11T13:02:20.321Z"
                time_2: "1111-11-11T13:02:20.321+03:00"
                time_tz: "1111-11-11T13:02:20.321Z"
                time_tz_2: "1111-11-11T13:02:20.321+03:00"
                ts: "2016-09-24T14:01:30.213Z"
                ts_2: "2016-09-24T14:01:30.213+03:00"
                ts_tz: "2016-09-24T14:01:30.213Z"
                ts_tz_2: "2016-09-24T14:01:30.213+03:00"
            }}
        }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyParent { id child { date date_2 time time_2 time_tz time_tz_2 ts ts_2 ts_tz ts_tz_2 } } }"#),
          @r###"{"data":{"findManyParent":[{"id":1,"child":{"date":"2016-09-24T00:00:00.000Z","date_2":"2016-09-23T00:00:00.000Z","time":"1970-01-01T13:02:20.321Z","time_2":"1970-01-01T10:02:20.321Z","time_tz":"1970-01-01T13:02:20.321Z","time_tz_2":"1970-01-01T10:02:20.321Z","ts":"2016-09-24T14:01:30.213Z","ts_2":"2016-09-24T11:01:30.213Z","ts_tz":"2016-09-24T14:01:30.213Z","ts_tz_2":"2016-09-24T11:01:30.213Z"}}]}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneParent(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}

#[test_suite(only(Postgres))]
mod postgres_decimal {
    fn schema_decimal() -> String {
        let schema = indoc! {
            r#"
            model Parent {
                #id(id, Int, @id)

                childId Int? @unique
                child Child? @relation(fields: [childId], references: [id])
            }

            model Child {
              #id(id, Int, @id)

              float    Float   @test.Real
              dfloat   Float   @test.DoublePrecision
              decFloat Decimal @test.Decimal(2, 1)
              money    Decimal @test.Money

              parent Parent?
            }"#
        };

        schema.to_owned()
    }

    // "Postgres native decimal types" should "work"
    #[connector_test(schema(schema_decimal))]
    async fn native_decimal_types(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
            id: 1,
            child: { create: {
                id: 1,
                float: 1.1,
                dfloat: 2.2,
                decFloat: 3.1234,
                money: 3.51,
            }}
        }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyParent { id child { float dfloat decFloat money } } }"#),
          @r###"{"data":{"findManyParent":[{"id":1,"child":{"float":1.1,"dfloat":2.2,"decFloat":"3.1","money":"3.51"}}]}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneParent(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}

#[test_suite(only(Postgres))]
mod postgres_string {
    fn schema_string() -> String {
        let schema = indoc! {
            r#"
            model Parent {
                #id(id, Int, @id)

                childId Int? @unique
                child Child? @relation(fields: [childId], references: [id])
            }

            model Child {
              #id(id, Int, @id)
              char  String @test.Char(10)
              vChar String @test.VarChar(11)
              text  String @test.Text
              bit   String @test.Bit(4)
              vBit  String @test.VarBit(5)
              uuid  String @test.Uuid
              ip    String @test.Inet

              parent Parent?
            }"#
        };

        schema.to_owned()
    }

    // "Postgres native string types" should "work"
    #[connector_test(schema(schema_string))]
    async fn native_string(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
            id: 1,
            child: { create: {
                id: 1,
                char: "1234567890"
                vChar: "12345678910"
                text: "text"
                bit: "1010"
                vBit: "00110"
                uuid: "123e4567-e89b-12d3-a456-426614174000"
                ip: "127.0.0.1"
            }}
          }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyParent {
            id
            child {
              char
              vChar
              text
              bit
              vBit
              uuid
              ip
            }
        }}"#),
          @r###"{"data":{"findManyParent":[{"id":1,"child":{"char":"1234567890","vChar":"12345678910","text":"text","bit":"1010","vBit":"00110","uuid":"123e4567-e89b-12d3-a456-426614174000","ip":"127.0.0.1"}}]}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneParent(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}

#[test_suite(
    schema(schema),
    only(Postgres("9", "10", "11", "12", "13", "14", "15", "pg.js.wasm", "neon.js.wasm"))
)]
mod postgres_others {
    fn schema_other_types() -> String {
        let schema = indoc! {
            r#"
            model Parent {
              #id(id, Int, @id)

              childId Int? @unique
              child Child? @relation(fields: [childId], references: [id])
            }

            model Child {
              #id(id, Int, @id)
              bool  Boolean @test.Boolean
              byteA Bytes   @test.ByteA
              json  Json    @test.Json
              jsonb Json    @test.JsonB

              parent Parent?
            }"#
        };

        schema.to_owned()
    }

    // "Other Postgres native types" should "work"
    #[connector_test(schema(schema_other_types))]
    async fn native_other_types(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
            id: 1,
            child: {
                create: {
                    id: 1,
                    bool: true
                    byteA: "dGVzdA=="
                    json: "{}"
                    jsonb: "{\"a\": \"b\"}"
                }
            }
          }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyParent { id child { id bool byteA json jsonb } } }"#),
          @r###"{"data":{"findManyParent":[{"id":1,"child":{"id":1,"bool":true,"byteA":"dGVzdA==","json":"{}","jsonb":"{\"a\":\"b\"}"}}]}}"###
        );

        Ok(())
    }

    fn schema_xml() -> String {
        let schema = indoc! {
            r#"
            model Parent {
              #id(id, Int, @id)

              childId Int? @unique
              child Child? @relation(fields: [childId], references: [id])
            }

            model Child {
              #id(id, Int, @id)
              xml String @test.Xml

              parent Parent?
            }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_xml), only(Postgres))]
    async fn native_xml(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
            id: 1,
            child: {
                create: {
                    id: 1,
                    xml: "<salad>wurst</salad>"
                }
            }
        }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyParent { id child { xml } } }"#),
          @r###"{"data":{"findManyParent":[{"id":1,"child":{"xml":"<salad>wurst</salad>"}}]}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneParent(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}
