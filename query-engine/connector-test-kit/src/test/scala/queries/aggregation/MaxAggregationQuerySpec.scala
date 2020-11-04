package queries.aggregation

import org.scalatest.{FlatSpec, Matchers}
import play.api.libs.json.JsNull
import util._

class MaxAggregationQuerySpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = SchemaDsl.fromStringV11() {
    """model Item {
      |  id    String @id @default(cuid())
      |  float Float
      |  int   Int
      |  dec   Decimal
      |  bInt  BigInt
      |}
    """.stripMargin
  }

  override protected def beforeEach(): Unit = {
    super.beforeEach()
    database.setup(project)
  }

  def createItem(float: Double, int: Int, dec: String, bInt: String, id: Option[String] = None) = {
    val idString = id match {
      case Some(i) => s"""id: "$i","""
      case None    => ""
    }

    server.query(
      s"""mutation {
         |  createItem(data: { $idString float: $float, int: $int, dec: $dec, bInt: $bInt }) {
         |    id
         |  }
         |}""".stripMargin,
      project
    )
  }

  "Calculating min with no records in the database" should "return zero" in {
    val result = server.query(
      s"""{
         |  aggregateItem {
         |    max {
         |      float
         |      int
         |      dec
         |      bInt
         |    }
         |  }
         |}""".stripMargin,
      project
    )

    result.pathAsFloat("data.aggregateItem.max.float") should be(0.0)
    result.pathAsString("data.aggregateItem.max.dec") should be("0")

    result.pathAsInt("data.aggregateItem.max.int") should be(0)
    result.pathAsString("data.aggregateItem.max.bInt") should be("0")
  }

  "Calculating min with some records in the database" should "return the correct maxima" in {
    createItem(5.5, 5, "5.5", "5")
    createItem(4.5, 10, "4.5", "10")

    val result = server.query(
      s"""{
         |  aggregateItem {
         |    max {
         |      float
         |      int
         |      dec
         |      bInt
         |    }
         |  }
         |}""".stripMargin,
      project
    )

    result.pathAsDouble("data.aggregateItem.max.float") should be(5.5)
    result.pathAsString("data.aggregateItem.max.dec") should be("5.5")

    result.pathAsDouble("data.aggregateItem.max.int") should be(10)
    result.pathAsString("data.aggregateItem.max.bInt") should be("10")
  }

  "Calculating min with all sorts of query arguments" should "work" in {
    createItem(5.5, 5, "5.5", "5", Some("1"))
    createItem(4.5, 10, "4.5", "10", Some("2"))
    createItem(1.5, 2, "1.5", "2", Some("3"))
    createItem(0.0, 1, "0.0", "1", Some("4"))

    var result = server.query(
      """{
        |  aggregateItem(take: 2) {
        |    max {
        |      float
        |      int
        |      dec
        |      bInt
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.pathAsDouble("data.aggregateItem.max.float") should be(5.5)
    result.pathAsString("data.aggregateItem.max.dec") should be("5.5")

    result.pathAsInt("data.aggregateItem.max.int") should be(10)
    result.pathAsString("data.aggregateItem.max.bInt") should be("10")

    result = server.query(
      """{
        |  aggregateItem(take: 5) {
        |    max {
        |      float
        |      int
        |      dec
        |      bInt
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.pathAsDouble("data.aggregateItem.max.float") should be(5.5)
    result.pathAsString("data.aggregateItem.max.dec") should be("5.5")

    result.pathAsInt("data.aggregateItem.max.int") should be(10)
    result.pathAsString("data.aggregateItem.max.bInt") should be("10")

    result = server.query(
      """{
        |  aggregateItem(take: -5) {
        |    max {
        |      float
        |      int
        |      dec
        |      bInt
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.pathAsDouble("data.aggregateItem.max.float") should be(5.5)
    result.pathAsString("data.aggregateItem.max.dec") should be("5.5")

    result.pathAsInt("data.aggregateItem.max.int") should be(10)
    result.pathAsString("data.aggregateItem.max.bInt") should be("10")

    result = server.query(
      """{
        |  aggregateItem(where: { id: { gt: "2" }}) {
        |    max {
        |      float
        |      int
        |      dec
        |      bInt
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.pathAsDouble("data.aggregateItem.max.float") should be(1.5)
    result.pathAsString("data.aggregateItem.max.dec") should be("1.5")

    result.pathAsInt("data.aggregateItem.max.int") should be(2)
    result.pathAsString("data.aggregateItem.max.bInt") should be("2")

    result = server.query(
      """{
        |  aggregateItem(skip: 2) {
        |    max {
        |      float
        |      int
        |      dec
        |      bInt
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.pathAsDouble("data.aggregateItem.max.float") should be(1.5)
    result.pathAsString("data.aggregateItem.max.dec") should be("1.5")

    result.pathAsInt("data.aggregateItem.max.int") should be(2)
    result.pathAsString("data.aggregateItem.max.bInt") should be("2")

    result = server.query(
      s"""{
        |  aggregateItem(cursor: { id: "3" }) {
        |    max {
        |      float
        |      int
        |      dec
        |      bInt
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.pathAsDouble("data.aggregateItem.max.float") should be(1.5)
    result.pathAsString("data.aggregateItem.max.dec") should be("1.5")

    result.pathAsInt("data.aggregateItem.max.int") should be(2)
    result.pathAsString("data.aggregateItem.max.bInt") should be("2")
  }
}
