package queries.aggregation

import org.scalatest.{FlatSpec, Matchers}
import play.api.libs.json.JsNull
import util._

// RS: Ported
class MinAggregationQuerySpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = SchemaDsl.fromStringV11() {
    """model Item {
      |  id    String @id @default(cuid())
      |  float Float
      |  int   Int
      |  dec   Decimal
      |  bInt  BigInt
      |  s     String
      |}
    """.stripMargin
  }

  override protected def beforeEach(): Unit = {
    super.beforeEach()
    database.setup(project)
  }

  def createItem(float: Double, int: Int, dec: String, bInt: String, s: String, id: Option[String] = None) = {
    val idString = id match {
      case Some(i) => s"""id: "$i","""
      case None    => ""
    }

    server.query(
      s"""mutation {
         |  createItem(data: { $idString float: $float, int: $int, dec: $dec, bInt: $bInt, s: "$s" }) {
         |    id
         |  }
         |}""".stripMargin,
      project
    )
  }

  "Calculating min with no records in the database" should "return null" in {
    val result = server.query(
      s"""{
         |  aggregateItem {
         |    _min {
         |      float
         |      int
         |      dec
         |      bInt
         |      s
         |    }
         |  }
         |}""".stripMargin,
      project
    )

    result.pathAsJsValue("data.aggregateItem._min.float") should be(JsNull)
    result.pathAsJsValue("data.aggregateItem._min.dec") should be(JsNull)

    result.pathAsJsValue("data.aggregateItem._min.int") should be(JsNull)
    result.pathAsJsValue("data.aggregateItem._min.bInt") should be(JsNull)

    result.pathAsJsValue("data.aggregateItem._min.s") should be(JsNull)
  }

  "Calculating min with some records in the database" should "return the correct minima" in {
    createItem(5.5, 5, "5.5", "5", "a")
    createItem(4.5, 10, "4.5", "10", "b")

    val result = server.query(
      s"""{
         |  aggregateItem {
         |    _min {
         |      float
         |      int
         |      dec
         |      bInt
         |      s
         |    }
         |  }
         |}""".stripMargin,
      project
    )

    result.pathAsDouble("data.aggregateItem._min.float") should be(4.5)
    result.pathAsString("data.aggregateItem._min.dec") should be("4.5")

    result.pathAsInt("data.aggregateItem._min.int") should be(5)
    result.pathAsString("data.aggregateItem._min.bInt") should be("5")

    result.pathAsString("data.aggregateItem._min.s") should be("a")
  }

  "Calculating min with all sorts of query arguments" should "work" in {
    createItem(5.5, 5, "5.5", "5", "2", Some("1"))
    createItem(4.5, 10, "4.5", "10", "f", Some("2"))
    createItem(1.5, 2, "1.5", "2", "z", Some("3"))
    createItem(0.0, 1, "0.0", "1", "g", Some("4"))

    var result = server.query(
      """{
        |  aggregateItem(take: 2) {
        |    _min {
        |      float
        |      int
        |      dec
        |      bInt
        |      s
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.pathAsDouble("data.aggregateItem._min.float") should be(4.5)
    result.pathAsString("data.aggregateItem._min.dec") should be("4.5")

    result.pathAsInt("data.aggregateItem._min.int") should be(5)
    result.pathAsString("data.aggregateItem._min.bInt") should be("5")

    result.pathAsString("data.aggregateItem._min.s") should be("2")

    result = server.query(
      """{
        |  aggregateItem(take: 5) {
        |    _min {
        |      float
        |      int
        |      dec
        |      bInt
        |      s
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.pathAsDouble("data.aggregateItem._min.float") should be(0.0)
    result.pathAsString("data.aggregateItem._min.dec") should be("0")

    result.pathAsInt("data.aggregateItem._min.int") should be(1)
    result.pathAsString("data.aggregateItem._min.bInt") should be("1")

    result.pathAsString("data.aggregateItem._min.s") should be("2")

    result = server.query(
      """{
        |  aggregateItem(take: -5) {
        |    _min {
        |      float
        |      int
        |      dec
        |      bInt
        |      s
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.pathAsDouble("data.aggregateItem._min.float") should be(0.0)
    result.pathAsString("data.aggregateItem._min.dec") should be("0")

    result.pathAsInt("data.aggregateItem._min.int") should be(1)
    result.pathAsString("data.aggregateItem._min.bInt") should be("1")

    result.pathAsString("data.aggregateItem._min.s") should be("2")

    result = server.query(
      """{
        |  aggregateItem(where: { id: { gt: "2" }}) {
        |    _min {
        |      float
        |      int
        |      dec
        |      bInt
        |      s
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.pathAsDouble("data.aggregateItem._min.float") should be(0.0)
    result.pathAsString("data.aggregateItem._min.dec") should be("0")

    result.pathAsInt("data.aggregateItem._min.int") should be(1)
    result.pathAsString("data.aggregateItem._min.bInt") should be("1")

    result.pathAsString("data.aggregateItem._min.s") should be("g")

    result = server.query(
      """{
        |  aggregateItem(skip: 2) {
        |    _min {
        |      float
        |      int
        |      dec
        |      bInt
        |      s
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.pathAsDouble("data.aggregateItem._min.float") should be(0.0)
    result.pathAsString("data.aggregateItem._min.dec") should be("0")

    result.pathAsInt("data.aggregateItem._min.int") should be(1)
    result.pathAsString("data.aggregateItem._min.bInt") should be("1")

    result.pathAsString("data.aggregateItem._min.s") should be("g")

    result = server.query(
      s"""{
        |  aggregateItem(cursor: { id: "3" }) {
        |    _min {
        |      float
        |      int
        |      dec
        |      bInt
        |      s
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.pathAsDouble("data.aggregateItem._min.float") should be(0.0)
    result.pathAsString("data.aggregateItem._min.dec") should be("0")

    result.pathAsInt("data.aggregateItem._min.int") should be(1)
    result.pathAsString("data.aggregateItem._min.bInt") should be("1")

    result.pathAsString("data.aggregateItem._min.s") should be("g")
  }
}
