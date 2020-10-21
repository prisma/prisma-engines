package writes.dataTypes.native_types

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorTag.MySqlConnectorTag
import util._

class MySqlNativeTypesSpec extends FlatSpec with Matchers with ApiSpecBase with ConnectorAwareTest {
  override def runOnlyForConnectors: Set[ConnectorTag] = Set(MySqlConnectorTag)

  "MySQL native int types" should "work" in {
    // MySQL only allows one autoinc column, so loop through all to test them.
    for ((fieldName, annotation) <- Seq(("inc_int", "@test.Int"),
                                        ("inc_sInt", "@test.SmallInt"),
                                        ("inc_mInt", "@test.MediumInt"),
                                        ("inc_bInt", "@test.BigInt"))) {

      val project = ProjectDsl.fromString {
        s"""
        |model Model {
        |  $fieldName Int @id @default(autoincrement()) $annotation
        |  int      Int   @test.Int
        |  sInt     Int   @test.SmallInt
        |  mInt     Int   @test.MediumInt
        |  bInt     Int   @test.BigInt
        |}"""
      }

      println(project.dataModel)
      database.setup(project)

      val res = server.query(
        s"""
           |mutation {
           |  createOneModel(
           |    data: {
           |      int: 2147483647
           |      sInt: 32767
           |      mInt: 8388607
           |      bInt: 5294967295
           |    }
           |  ) {
           |    int
           |    sInt
           |    mInt
           |    bInt
           |    $fieldName
           |  }
           |}""".stripMargin,
        project,
        legacy = false
      )

      res.toString should be(s"""{"data":{"createOneModel":{"int":2147483647,"sInt":32767,"mInt":8388607,"bInt":5294967295,"$fieldName":1}}}""")
    }
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
       |      time: "2016-09-24T13:14:15.123Z"
       |      dtime: "2016-09-24T12:29:32.342Z"
       |      ts: "2016-09-24T12:29:32.342Z"
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

    res.toString should be(
      """{"data":{"createOneModel":{"date":"2016-09-24T00:00:00+00:00","time":"1970-01-01T13:14:15.123+00:00","dtime":"2016-09-24T12:29:32+00:00","ts":"2016-09-24T12:29:32+00:00","year":1973}}}""")
  }

  "MySQL native binary types" should "work" in {
    val project = ProjectDsl.fromString {
      """
        |model Model {
        |  id    String @id @default(cuid())
        |  bit   Bytes @test.Bit(8)
        |  bin   Bytes @test.Binary(4)
        |  vBin  Bytes @test.VarBinary(5)
        |  blob  Bytes @test.Blob
        |  tBlob Bytes @test.TinyBlob
        |  mBlob Bytes @test.MediumBlob
        |  lBlob Bytes @test.LongBlob
        |}"""
    }

    database.setup(project)

    val res = server.query(
      s"""
         |mutation {
         |  createOneModel(
         |    data: {
         |      bit: "dA=="
         |      bin: "dGVzdA=="
         |      vBin: "dGVzdA=="
         |      blob: "dGVzdA=="
         |      tBlob: "dGVzdA=="
         |      mBlob: "dGVzdA=="
         |      lBlob: "dGVzdA=="
         |    }
         |  ) {
         |    bit
         |    bin
         |    vBin
         |    blob
         |    tBlob
         |    mBlob
         |    lBlob
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    res.toString should be(
      """{"data":{"createOneModel":{"bit":"dA==","bin":"dGVzdA==","vBin":"dGVzdA==","blob":"dGVzdA==","tBlob":"dGVzdA==","mBlob":"dGVzdA==","lBlob":"dGVzdA=="}}}""")
  }

  "Other MySQL native types" should "work" in {
    val project = ProjectDsl.fromString {
      """
        |model Model {
        |  id   String  @id @default(cuid())
        |  tInt Boolean @test.TinyInt
        |}"""
    }

    database.setup(project)

    val res = server.query(
      s"""
         |mutation {
         |  createOneModel(
         |    data: {
         |      tInt: true
         |    }
         |  ) {
         |    tInt
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    res.toString should be("""{"data":{"createOneModel":{"tInt":true}}}""")
  }
}
