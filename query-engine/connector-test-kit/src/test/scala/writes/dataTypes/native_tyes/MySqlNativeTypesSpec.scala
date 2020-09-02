package writes.dataTypes.native_tyes

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorTag.MySqlConnectorTag
import util._

class MySqlNativeTypesSpec extends FlatSpec with Matchers with ApiSpecBase with ConnectorAwareTest {
  override def runOnlyForConnectors: Set[ConnectorTag] = Set(MySqlConnectorTag)

  "MySQL native types" should "work" in {
    val project = ProjectDsl.fromString {
      """
        |model Model {
        |  id  String @id @default(cuid())
        |
        |  int  Int @test.Int
        |  sInt Int @test.SmallInt
        |  tInt Int @test.TinyInt
        |  mInt Int @test.MediumInt
        |  bInt Int @test.BigInt
        |
        |  float  Float @test.Float
        |  dfloat Float @test.Double
        |
        |  decFloat Decimal @test.Decimal(2, 1)
        |  numFloat Decimal @test.Numeric(2, 1)
        |
        |  char  String @test.Char(10)
        |  vChar String @test.VarChar(11)
        |  tText String @test.TinyText
        |  text  String @test.Text
        |  mText String @test.MediumText
        |  ltext String @test.LongText
        |
        |  date  DateTime @test.Date
        |  time  DateTime @test.Time(5)
        |  dtime DateTime @test.Datetime
        |  ts    DateTime @test.Timestamp
        |  year  Int @test.Year
        |}"""
//      json Json @test.JSON
    }

    database.setup(project)

    var res = server.query(
      s"""
         |mutation {
         |  createOneModel(
         |    data: {
         |      int: 4294967296
         |      sInt: 65536
         |      tInt: 127
         |      mInt: 16777216
         |      bInt: 4294967297
         |      float: 1.1
         |      dfloat: 2.2
         |      decFloat: 3.1234
         |      numFloat: 4.1234
         |      char: "1234567890"
         |      vChar: "12345678910"
         |      tText: "tiny text"
         |      text: "text"
         |      mText: "medium text"
         |      ltext: "long text"
         |      date: "1973-12-30"
         |      time: "15:30:00"
         |      dtime: "1973-12-30 15:30:00"
         |      ts: "19731230153000"
         |      year: 1973
         |    }
         |  ) {
         |    id
         |    int
         |    sInt
         |    tInt
         |    mInt
         |    bInt
         |    float
         |    dfloat
         |    decFloat
         |    numFloat
         |    char
         |    vChar
         |    tText
         |    text
         |    mText
         |    ltext
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
}
