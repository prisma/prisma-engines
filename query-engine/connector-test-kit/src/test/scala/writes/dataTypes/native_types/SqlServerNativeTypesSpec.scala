package writes.dataTypes.native_types

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorTag.MsSqlConnectorTag
import util.{ApiSpecBase, ConnectorAwareTest, ConnectorTag, ProjectDsl}

// RS: Ported
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
           |  int  Int     @test.Int
           |  sInt Int     @test.SmallInt
           |  tInt Int     @test.TinyInt
           |  bInt BigInt  @test.BigInt
           |  bit  Int     @test.Bit
           |  bool Boolean @test.Bit
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
           |      bit: 1
           |      bool: false
           |    }
           |  ) {
           |    int
           |    sInt
           |    tInt
           |    bInt
           |    bit
           |    bool
           |    $fieldName
           |  }
           |}""".stripMargin,
        project,
        legacy = false
      )

      if (intType == "BigInt") {
        res should be(
          s"""{"data":{"createOneModel":{"int":2147483647,"sInt":32767,"tInt":12,"bInt":"5294967295","bit": 1,"bool": false,"$fieldName":"1"}}}""".parseJson)
      } else {
        res should be(
          s"""{"data":{"createOneModel":{"int":2147483647,"sInt":32767,"tInt":12,"bInt":"5294967295","bit": 1,"bool": false,"$fieldName":1}}}""".parseJson)
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
        |  decFloat2  Decimal @test.Decimal(10, 6)
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
         |      decFloat2: "4.12345"
         |    }
         |  ) {
         |    float
         |    dfloat
         |    money
         |    smallMoney
         |    decFloat
         |    decFloat2
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    // decFloat is cut due to precision
    res should be(
      """{"data":{"createOneModel":{"float":1.1,"dfloat":2.2,"money":22.14,"smallMoney":22.12,"decFloat":"3.1","decFloat2":"4.12345"}}}""".parseJson)
  }

  "SQL Server native string types" should "work" in {
    val project = ProjectDsl.fromString {
      """
        |model Model {
        |  id     String @id @default(uuid()) @test.UniqueIdentifier
        |  char   String @test.Char(10)
        |  nchar  String @test.NChar(10)
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
         |      nchar: "1234567890"
         |      vChar: "12345678910"
         |      nVChar: "教育漢字教育漢字"
         |      text: "text"
         |      nText: "教育漢字"
         |    }
         |  ) {
         |    char
         |    nchar
         |    vChar
         |    nVChar
         |    text
         |    nText
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    res should be(
      """{"data":{"createOneModel":{"char":"1234567890","nchar":"1234567890","vChar":"12345678910","nVChar":"教育漢字教育漢字","text":"text","nText":"教育漢字"}}}""".parseJson)
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

    res should be(
      """{"data":{"createOneModel":{"date":"2016-09-24T00:00:00.000Z","time":"1970-01-01T13:14:15.123Z","dtime":"2016-09-24T12:29:32.343Z","dtime2":"2016-09-24T12:29:32.342Z","dtoff":"2016-09-24T12:29:32.342Z","small":"2016-09-24T12:30:00.000Z"}}}""".parseJson)
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

    res should be("""{"data":{"createOneModel":{"bin":"dGVzdA==","vBin":"dGVzdA==","image":"dGVzdA=="}}}""".parseJson)
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

    res should be("""{"data":{"createOneModel":{"xml":"<meow>purr</meow>","uuid":"ab309dfd-d041-4110-b162-75d7b95fe989"}}}""".parseJson)
  }

  "Sql server native fixed-size char type" should "be handled correctly wrt. padding for comparisons" in {
    val project = ProjectDsl.fromString {
      """
        |model ModelA {
        |  id   String  @id @test.Char(16)
        |  b_id String? @test.Char(16)
        |  b    ModelB? @relation(fields: [b_id], references: [id])
        |}
        |
        |model ModelB {
        |  id String @id @test.Char(16)
        |  a  ModelA?
        |}
        |"""
    }

    database.setup(project)

    val res = server.query(
      s"""
         |mutation {
         |  createOneModelA(data: {
         |    id: "1234"
         |     b: { create: { id: "4321" } }
         |  }) {
         |    id
         |    b { id }
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    // This is correct - sql server returns padded strings (as opposed to MySQL for example, where it's trimmed).
    res.toString should be("""{"data":{"createOneModelA":{"id":"1234            ","b":{"id":"4321            "}}}}""")
  }

  "Sql server native fixed-size nchar type" should "be handled correctly wrt. padding for comparisons" in {
    val project = ProjectDsl.fromString {
      """
        |model ModelA {
        |  id   String  @id @test.NChar(16)
        |  b_id String? @test.NChar(16)
        |  b    ModelB? @relation(fields: [b_id], references: [id])
        |}
        |
        |model ModelB {
        |  id String @id @test.NChar(16)
        |  a  ModelA?
        |}
        |"""
    }

    database.setup(project)

    val res = server.query(
      s"""
         |mutation {
         |  createOneModelA(data: {
         |    id: "1234"
         |     b: { create: { id: "4321" } }
         |  }) {
         |    id
         |    b { id }
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    // This is correct - sql server returns padded strings (as opposed to MySQL for example, where it's trimmed).
    res.toString should be("""{"data":{"createOneModelA":{"id":"1234            ","b":{"id":"4321            "}}}}""")
  }
}
