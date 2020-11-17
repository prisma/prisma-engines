package writes.dataTypes.native_types

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorTag.MsSqlConnectorTag
import util.{ApiSpecBase, ConnectorAwareTest, ConnectorTag, ProjectDsl}

class SqlServerNativeTypesSpec extends FlatSpec with Matchers with ApiSpecBase with ConnectorAwareTest {
  override def runOnlyForConnectors: Set[ConnectorTag] = Set(MsSqlConnectorTag)

  "SQL Server native int types" should "work" in {
    for ((fieldName, intType, annotation) <- Seq(("inc_int", "Int", "@test.Int"),
                                                 ("inc_tInt", "Int", "@test.TinyInt"),
                                                 ("inc_sInt", "Int", "@test.SmallInt"),
                                                 ("inc_bInt", "BigInt", "@test.BigInt"))) {

      val project = ProjectDsl.fromString {
        s"""
           |model Model {
           |  $fieldName $intType @id @default(autoincrement()) $annotation
           |  int  Int    @test.Int
           |  sInt Int    @test.SmallInt
           |  tInt Int    @test.TinyInt
           |  bInt BigInt @test.BigInt
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
           |      tInt: 12
           |      bInt: 5294967295
           |    }
           |  ) {
           |    int
           |    sInt
           |    tInt
           |    bInt
           |    $fieldName
           |  }
           |}""".stripMargin,
        project,
        legacy = false
      )

      if (intType == "BigInt") {
        res.toString should be(s"""{"data":{"createOneModel":{"int":2147483647,"sInt":32767,"tInt":12,"bInt":"5294967295","$fieldName":"1"}}}""")
      } else {
        res.toString should be(s"""{"data":{"createOneModel":{"int":2147483647,"sInt":32767,"tInt":12,"bInt":"5294967295","$fieldName":1}}}""")
      }
    }
  }

  "SQL Server native decimal types" should "work" in {
    val project = ProjectDsl.fromString {
      """
        |model Model {
        |  id         String  @id @default(uuid()) @test.UniqueIdentifier
        |  float      Float   @test.Real
        |  dfloat     Float   @test.Float(53)
        |  money      Float   @test.Money
        |  smallMoney Float   @test.SmallMoney
        |  decFloat   Decimal @test.Decimal(2, 1)
        |  numFloat   Decimal @test.Numeric(10, 6)
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
         |      money: 22.14
         |      smallMoney: 22.12
         |      decFloat: 3.1234
         |      numFloat: "4.12345"
         |    }
         |  ) {
         |    float
         |    dfloat
         |    money
         |    smallMoney
         |    decFloat
         |    numFloat
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    // decFloat is cut due to precision
    res.toString should be("""{"data":{"createOneModel":{"float":1.1,"dfloat":2.2,"money":22.14,"smallMoney":22.12,"decFloat":"3.1","numFloat":"4.12345"}}}""")
  }

  "SQL Server native string types" should "work" in {
    val project = ProjectDsl.fromString {
      """
        |model Model {
        |  id     String @id @default(uuid()) @test.UniqueIdentifier
        |  char   String @test.Char(10)
        |  vChar  String @test.VarChar(Max)
        |  nVChar String @test.NVarChar(1000)
        |  text   String @test.Text
        |  nText  String @test.NText
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
         |      nVChar: "教育漢字教育漢字"
         |      text: "text"
         |      nText: "教育漢字"
         |    }
         |  ) {
         |    char
         |    vChar
         |    nVChar
         |    text
         |    nText
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    res.toString should be("""{"data":{"createOneModel":{"char":"1234567890","vChar":"12345678910","nVChar":"教育漢字教育漢字","text":"text","nText":"教育漢字"}}}""")
  }

  "SQL Server native date types" should "work" in {
    val project = ProjectDsl.fromString {
      """
        |model Model {
        |  id     String   @id @default(uuid()) @test.UniqueIdentifier
        |  date   DateTime @test.Date
        |  time   DateTime @test.Time
        |  dtime  DateTime @test.DateTime
        |  dtime2 DateTime @test.DateTime2
        |  dtoff  DateTime @test.DateTimeOffset
        |  small  DateTime @test.SmallDateTime
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
         |      dtime2: "2016-09-24T12:29:32.342Z"
         |      dtoff: "2016-09-24T12:29:32.342Z"
         |      small: "2016-09-24T12:29:32.342Z"
         |    }
         |  ) {
         |    date
         |    time
         |    dtime
         |    dtime2
         |    dtoff
         |    small
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    res.toString should be(
      """{"data":{"createOneModel":{"date":"2016-09-24T00:00:00+00:00","time":"1970-01-01T13:14:15.123+00:00","dtime":"2016-09-24T12:29:32.343333333+00:00","dtime2":"2016-09-24T12:29:32.342+00:00","dtoff":"2016-09-24T12:29:32.342+00:00","small":"2016-09-24T12:30:00+00:00"}}}""")
  }

  "SQL Server native binary types" should "work" in {
    val project = ProjectDsl.fromString {
      """
        |model Model {
        |  id    String @id @default(uuid()) @test.UniqueIdentifier
        |  bin   Bytes @test.Binary(4)
        |  vBin  Bytes @test.VarBinary(Max)
        |  image Bytes @test.Image
        |}"""
    }

    database.setup(project)

    val res = server.query(
      s"""
         |mutation {
         |  createOneModel(
         |    data: {
         |      bin: "dGVzdA=="
         |      vBin: "dGVzdA=="
         |      image: "dGVzdA=="
         |    }
         |  ) {
         |    bin
         |    vBin
         |    image
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    res.toString should be("""{"data":{"createOneModel":{"bin":"dGVzdA==","vBin":"dGVzdA==","image":"dGVzdA=="}}}""")
  }

  "Other SQL Server native types" should "work" in {
    val project = ProjectDsl.fromString {
      """
        |model Model {
        |  id   String  @id @default(cuid())
        |  xml  String  @test.Xml
        |  uuid String  @test.UniqueIdentifier
        |}"""
    }

    database.setup(project)

    val res = server.query(
      s"""
         |mutation {
         |  createOneModel(
         |    data: {
         |      xml: "<meow>purr</meow>"
         |      uuid: "ab309dfd-d041-4110-b162-75d7b95fe989"
         |    }
         |  ) {
         |    xml
         |    uuid
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    res.toString should be("""{"data":{"createOneModel":{"xml":"<meow>purr</meow>","uuid":"ab309dfd-d041-4110-b162-75d7b95fe989"}}}""")
  }
}
