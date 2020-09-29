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
        |  serial   Int    @default(autoincrement()) @test.Serial
        |  sSerial  Int    @default(autoincrement()) @test.SmallSerial
        |  bSerial  Int    @default(autoincrement()) @test.BigSerial
        |  inc_int  Int    @default(autoincrement()) @test.Integer
        |  inc_sInt Int    @default(autoincrement()) @test.SmallInt
        |  inc_bInt Int    @default(autoincrement()) @test.BigInt
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
         |    inc_sint
         |    inc_bint
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    res.toString should be("""{"data":{"createOneModel":{}}}""")
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
       |      numFloat: 4.12345
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
    res.toString should be("""{"data":{"createOneModel":{"float":1.1,"dfloat":2.2,"decFloat":3.1,"numFloat":4.12345}}}""")
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
         |      json: "{}"
         |      jsonb: "{\\"a\\": \\"b\\"}"
         |    }
         |  ) {
         |    bool
         |    json
         |    jsonb
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    res.toString should be("""{"data":{"createOneModel":{"bool":true,"json":"{}","jsonb":"{\"a\":\"b\"}"}}}""")
  }

  "Postgres native date types" should "work" ignore {
    val project = ProjectDsl.fromString {
      """
        |model Model {
        |  id     String   @id @default(cuid())
        |  date   DateTime @test.Date
        |  time   DateTime @test.Time
        |  timeTz DateTime @test.TimeWithTimeZone
        |  ts     DateTime @test.Timestamp
        |  tsTz   DateTime @test.TimestampWithTimeZone
        |}"""
    }

    database.setup(project)

    val res = server.query(
      s"""
       |mutation {
       |  createOneModel(
       |    data: {
       |      date: "2016-09-24T00:00:00.000Z"
       |      time: "0000-00-00T12:29:32.342Z"
       |      timeTz: "0000-00-00T12:29:32.342Z"
       |      ts: "19731230153000"
       |      tsTz: "19731230153000"
       |    }
       |  ) {
       |    date
       |    time
       |    timeTz
       |    ts
       |    tsTz
       |  }
       |}""".stripMargin,
      project,
      legacy = false
    )

    res.toString should be("""{"data":{"createOneModel":{"field":"{\"a\":\"b\"}"}}}""")
  }

  // XML
  // JSON?
  // Bytes
}
