package writes.dataTypes.native_types

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorTag.MySqlConnectorTag
import util._

class MySqlNativeTypesSpec extends FlatSpec with Matchers with ApiSpecBase with ConnectorAwareTest {
  override def runOnlyForConnectors: Set[ConnectorTag] = Set(MySqlConnectorTag)

  "MySQL native int types" should "work" in {
    val project = ProjectDsl.fromString {
      """
        |model Model {
        |  id   String @id @default(cuid())
        |  int  Int    @test.Int
        |  sInt Int    @test.SmallInt
        |  tInt Int    @test.TinyInt
        |  mInt Int    @test.MediumInt
        |  bInt Int    @test.BigInt
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
         |      tInt: 127
         |      mInt: 8388607
         |      bInt: 5294967295
         |    }
         |  ) {
         |    int
         |    sInt
         |    tInt
         |    mInt
         |    bInt
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    res.toString should be("""{"data":{"createOneModel":{"int":2147483647,"sInt":32767,"tInt":127,"mInt":8388607,"bInt":5294967295}}}""")
  }

  "MySQL native decimal types" should "work" in {
    val project = ProjectDsl.fromString {
      """
        |model Model {
        |  id       String  @id @default(cuid())
        |  float    Float   @test.Float
        |  dfloat   Float   @test.Double
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

  "MySQL native string types" should "work" in {
    val project = ProjectDsl.fromString {
      """
        |model Model {
        |  id    String @id @default(cuid())
        |  char  String @test.Char(10)
        |  vChar String @test.VarChar(11)
        |  tText String @test.TinyText
        |  text  String @test.Text
        |  mText String @test.MediumText
        |  ltext String @test.LongText
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
       |      tText: "tiny text"
       |      text: "text"
       |      mText: "medium text"
       |      ltext: "long text"
       |    }
       |  ) {
       |    char
       |    vChar
       |    tText
       |    text
       |    mText
       |    ltext
       |  }
       |}""".stripMargin,
      project,
      legacy = false
    )

    res.toString should be(
      """{"data":{"createOneModel":{"char":"1234567890","vChar":"12345678910","tText":"tiny text","text":"text","mText":"medium text","ltext":"long text"}}}""")
  }

  "MySQL native date types" should "work" in {
    val project = ProjectDsl.fromString {
      """
        |model Model {
        |  id    String   @id @default(cuid())
        |  date  DateTime @test.Date
        |  time  DateTime @test.Time(5)
        |  dtime DateTime @test.Datetime
        |  ts    DateTime @test.Timestamp
        |  year  Int      @test.Year
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
       |      dtime: "2016-09-24T12:29:32.342Z"
       |      ts: "19731230153000"
       |      year: 1973
       |    }
       |  ) {
       |    date
       |    time
       |    dtime
       |    ts
       |    year
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
