use indoc::indoc;
use query_engine_tests::*;

#[test_suite(only(Mysql("8")))]
mod datetime {
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
                ts         DateTime @test.Timestamp(3)
                ts_2       DateTime @test.Timestamp(3)
                dt         DateTime @test.DateTime(3)
                dt_2       DateTime @test.DateTime(3)
                year       Int      @test.Year

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
                ts: "2016-09-24T14:01:30.213Z"
                ts_2: "2016-09-24T14:01:30.213+03:00"
                dt: "2016-09-24T14:01:30.213Z"
                dt_2: "2016-09-24T14:01:30.213+03:00",
                year: 2023
            }}
        }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyParent { id child { date date_2 time time_2 ts ts_2 dt dt_2 year } } }"#),
          @r###"{"data":{"findManyParent":[{"id":1,"child":{"date":"2016-09-24T00:00:00.000Z","date_2":"2016-09-23T00:00:00.000Z","time":"1970-01-01T13:02:20.321Z","time_2":"1970-01-01T10:02:20.321Z","ts":"2016-09-24T14:01:30.213Z","ts_2":"2016-09-24T11:01:30.213Z","dt":"2016-09-24T14:01:30.213Z","dt_2":"2016-09-24T11:01:30.213Z","year":2023}}]}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneParent(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}

#[test_suite(only(Mysql("8")))]
mod mysql_decimal {
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

              float    Float   @test.Float
              dfloat   Float   @test.Double
              decFloat Decimal @test.Decimal(2, 1)

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
                float: 1.1
                dfloat: 2.2
                decFloat: 3.1234
            }}
        }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyParent { id child { float dfloat decFloat } } }"#),
          @r###"{"data":{"findManyParent":[{"id":1,"child":{"float":1.1,"dfloat":2.2,"decFloat":"3.1"}}]}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneParent(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}

#[test_suite(only(Mysql("8")))]
mod mysql_string {
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
              tText String @test.TinyText
              text  String @test.Text
              mText String @test.MediumText
              ltext String @test.LongText

              parent Parent?
            }"#
        };

        schema.to_owned()
    }

    // "Mysql native string types" should "work"
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
                tText: "tiny text"
                text: "text"
                mText: "medium text"
                ltext: "long text"
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
              tText
              text
              mText
              ltext
            }
        }}"#),
          @r###"{"data":{"findManyParent":[{"id":1,"child":{"char":"1234567890","vChar":"12345678910","tText":"tiny text","text":"text","mText":"medium text","ltext":"long text"}}]}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneParent(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}

#[test_suite(only(MySql("8")))]
mod mysql_bytes {
    fn schema_bytes() -> String {
        let schema = indoc! {
            r#"
            model Parent {
                #id(id, Int, @id)

                childId Int? @unique
                child Child? @relation(fields: [childId], references: [id])
            }

            model Child {
              #id(id, Int, @id)
              bit   Bytes @test.Bit(8)
              bin   Bytes @test.Binary(4)
              vBin  Bytes @test.VarBinary(5)
              blob  Bytes @test.Blob
              tBlob Bytes @test.TinyBlob
              mBlob Bytes @test.MediumBlob
              lBlob Bytes @test.LongBlob

              parent Parent?
            }"#
        };

        schema.to_owned()
    }

    // "Mysql native bytes types" should "work"
    #[connector_test(schema(schema_bytes))]
    async fn native_bytes(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{
            id: 1,
            child: { create: {
                id: 1,
                bit: "dA=="
                bin: "dGVzdA=="
                vBin: "dGVzdA=="
                blob: "dGVzdA=="
                tBlob: "dGVzdA=="
                mBlob: "dGVzdA=="
                lBlob: "dGVzdA=="
            }}
          }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyParent {
            id
            child {
              bit
              bin
              vBin
              blob
              tBlob
              mBlob
              lBlob
            }
        }}"#),
          @r###"{"data":{"findManyParent":[{"id":1,"child":{"bit":"dA==","bin":"dGVzdA==","vBin":"dGVzdA==","blob":"dGVzdA==","tBlob":"dGVzdA==","mBlob":"dGVzdA==","lBlob":"dGVzdA=="}}]}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneParent(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
