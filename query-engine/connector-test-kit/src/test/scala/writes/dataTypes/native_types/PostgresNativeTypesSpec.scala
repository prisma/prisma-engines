package writes.dataTypes.native_types

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorTag.PostgresConnectorTag
import util._

class PostgresNativeTypesSpec extends FlatSpec with Matchers with ApiSpecBase with ConnectorAwareTest {
  override def runOnlyForConnectors: Set[ConnectorTag] = Set(PostgresConnectorTag)

  "Postgres native int types" should "work" in {
    val project = ProjectDsl.fromString {
      """
        |model Model {
        |  id       String @id @default(cuid())
        |  int      Int    @test.Integer
        |  sInt     Int    @test.SmallInt
        |  bInt     Int    @test.BigInt
        |  serial   Int    @test.Serial      @default(autoincrement())
        |  sSerial  Int    @test.SmallSerial @default(autoincrement())
        |  bSerial  Int    @test.BigSerial   @default(autoincrement())
        |  inc_int  Int    @test.Integer     @default(autoincrement())
        |  inc_sInt Int    @test.SmallInt    @default(autoincrement())
        |  inc_bInt Int    @test.BigInt      @default(autoincrement())
        |}"""
    }

    database.setup(project)

    val res = server.query(
      s"""
         |mutation {
         |  createOneModel(
         |    data: {
         |      int: 2147483647
         |      sInt: 32767
         |      bInt: 5294967295
         |    }
         |  ) {
         |    int
         |    sInt
         |    bInt
         |    serial
         |    sSerial
         |    bSerial
         |    inc_int
         |    inc_sInt
         |    inc_bInt
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    res.toString should be(
      """{"data":{"createOneModel":{"int":2147483647,"sInt":32767,"bInt":5294967295,"serial":1,"sSerial":1,"bSerial":1,"inc_int":1,"inc_sInt":1,"inc_bInt":1}}}""")
  }

  "Postgres native decimal types" should "work" in {
    val project = ProjectDsl.fromString {
      """
        |model Model {
        |  id       String  @id @default(cuid())
        |  float    Float   @test.Real
        |  dfloat   Float   @test.DoublePrecision
        |  decFloat Decimal @test.Decimal(2, 1)
        |  numFloat Decimal @test.Numeric(10, 6)
        |}"""
    }

    database.setup(project)

    val res = server.query(
      s"""
       |mutation {
       |  createOneModel(
       |    data: {
       |      float: 1.1
       |      dfloat: 2.2
       |      decFloat: 3.1234
       |      numFloat: "4.12345"
       |    }
       |  ) {
       |    float
       |    dfloat
       |    decFloat
       |    numFloat
       |  }
       |}""".stripMargin,
      project,
      legacy = false
    )

    // decFloat is cut due to precision
    res.toString should be("""{"data":{"createOneModel":{"float":1.1,"dfloat":2.2,"decFloat":"3.1","numFloat":"4.12345"}}}""")
  }

  "Postgres native string types" should "work" in {
    val project = ProjectDsl.fromString {
      """
        |model Model {
        |  id    String @id @default(cuid())
        |  char  String @test.Char(10)
        |  vChar String @test.VarChar(11)
        |  text  String @test.Text
        |  bit   String @test.Bit(4)
        |  vBit  String @test.VarBit(5)
        |  uuid  String @test.Uuid
        |}"""
    }

    database.setup(project)

    val res = server.query(
      s"""
       |mutation {
       |  createOneModel(
       |    data: {
       |      char: "1234567890"
       |      vChar: "12345678910"
       |      text: "text"
       |      bit: "1010"
       |      vBit: "00110"
       |      uuid: "123e4567-e89b-12d3-a456-426614174000"
       |    }
       |  ) {
       |    char
       |    vChar
       |    text
       |    bit
       |    vBit
       |    uuid
       |  }
       |}""".stripMargin,
      project,
      legacy = false
    )

    res.toString should be(
      """{"data":{"createOneModel":{"char":"1234567890","vChar":"12345678910","text":"text","bit":"1010","vBit":"00110","uuid":"123e4567-e89b-12d3-a456-426614174000"}}}""")
  }

  "Other Postgres native types" should "work" in {
    val project = ProjectDsl.fromString {
      """
        |model Model {
        |  id    String  @id @default(cuid())
        |  bool  Boolean @test.Boolean
        |  byteA Bytes   @test.ByteA
        |  xml   Xml     @test.Xml
        |  json  Json    @test.Json
        |  jsonb Json    @test.JsonB
        |}"""
    }

    database.setup(project)

    val res = server.query(
      s"""
         |mutation {
         |  createOneModel(
         |    data: {
         |      bool: true
         |      byteA: "dGVzdA=="
         |      xml: "<wurst>salat</wurst>"
         |      json: "{}"
         |      jsonb: "{\\"a\\": \\"b\\"}"
         |    }
         |  ) {
         |    bool
         |    byteA
         |    xml
         |    json
         |    jsonb
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    res.toString should be("""{"data":{"createOneModel":{"bool":true,"byteA":"dGVzdA==","xml":"<wurst>salat</wurst>","json":"{}","jsonb":"{\"a\":\"b\"}"}}}""")
  }

  "Postgres native date types" should "work" in {
    val project = ProjectDsl.fromString {
      """
        |model Model {
        |  id     String   @id @default(cuid())
        |  date   DateTime @test.Date
        |  time   DateTime @test.Time(3)
        |  ts     DateTime @test.Timestamp(3)
        |}"""
    }

    database.setup(project)

    val res = server.query(
      s"""
       |mutation {
       |  createOneModel(
       |    data: {
       |      date: "2016-09-24T00:00:00.000Z"
       |      time: "1111-11-11T13:02:20.321Z"
       |      ts: "2016-09-24T14:01:30.213Z"
       |    }
       |  ) {
       |    date
       |    time
       |    ts
       |  }
       |}""".stripMargin,
      project,
      legacy = false
    )

    res.toString should be(
      """{"data":{"createOneModel":{"date":"2016-09-24T00:00:00+00:00","time":"1970-01-01T13:02:20.321+00:00","ts":"2016-09-24T14:01:30.213+00:00"}}}""")
  }

  // XML
}
