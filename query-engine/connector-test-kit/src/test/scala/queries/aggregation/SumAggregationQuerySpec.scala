package queries.aggregation

import org.scalatest.{FlatSpec, Matchers}
import play.api.libs.json.JsNull
import util._

class SumAggregationQuerySpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = SchemaDsl.fromStringV11() {
    """model Item {
      |  id    String @id @default(cuid())
      |  float Float
      |  int   Int
      |  dec   Decimal
      |}
    """.stripMargin
  }

  override protected def beforeEach(): Unit = {
    super.beforeEach()
    database.setup(project)
  }

  def createItem(float: Double, int: Int, dec: String, id: Option[String] = None) = {
    val idString = id match {
      case Some(i) => s"""id: "$i","""
      case None    => ""
    }

    server.query(
      s"""mutation {
         |  createItem(data: { $idString float: $float, int: $int, dec: $dec }) {
         |    id
         |  }
         |}""".stripMargin,
      project
    )
  }

  "Summing with no records in the database" should "return zero" in {
    val result = server.query(
      s"""{
         |  aggregateItem {
         |    sum {
         |      float
         |      int
         |      dec
         |    }
         |  }
         |}""".stripMargin,
      project
    )

    result.pathAsFloat("data.aggregateItem.sum.float") should be(0.0)
    result.pathAsInt("data.aggregateItem.sum.int") should be(0)
    result.pathAsString("data.aggregateItem.sum.dec") should be("0")
  }

  "Summing with some records in the database" should "return the correct sum" in {
    createItem(5.5, 5, "5.5")
    createItem(4.5, 10, "4.5")

    val result = server.query(
      s"""{
         |  aggregateItem {
         |    sum {
         |      float
         |      int
         |      dec
         |    }
         |  }
         |}""".stripMargin,
      project
    )

    result.pathAsDouble("data.aggregateItem.sum.float") should be(10.0)
    result.pathAsInt("data.aggregateItem.sum.int") should be(15)
    result.pathAsString("data.aggregateItem.sum.dec") should be("10")
  }

  "Summing with all sorts of query arguments" should "work" in {
    createItem(5.5, 5, "5.5", Some("1"))
    createItem(4.5, 10, "4.5", Some("2"))
    createItem(1.5, 2, "1.5", Some("3"))
    createItem(0.0, 1, "0", Some("4"))

    var result = server.query(
      """{
        |  aggregateItem(take: 2) {
        |    sum {
        |      float
        |      int
        |      dec
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.pathAsDouble("data.aggregateItem.sum.float") should be(10.0)
    result.pathAsInt("data.aggregateItem.sum.int") should be(15)
    result.pathAsString("data.aggregateItem.sum.dec") should be("10")

    result = server.query(
      """{
        |  aggregateItem(take: 5) {
        |    sum {
        |      float
        |      int
        |      dec
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.pathAsDouble("data.aggregateItem.sum.float") should be(11.5)
    result.pathAsInt("data.aggregateItem.sum.int") should be(18)
    result.pathAsString("data.aggregateItem.sum.dec") should be("11.5")

    result = server.query(
      """{
        |  aggregateItem(take: -5) {
        |    sum {
        |      float
        |      int
        |      dec
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.pathAsDouble("data.aggregateItem.sum.float") should be(11.5)
    result.pathAsInt("data.aggregateItem.sum.int") should be(18)
    result.pathAsString("data.aggregateItem.sum.dec") should be("11.5")

    result = server.query(
      """{
        |  aggregateItem(where: { id: { gt: "2" }}) {
        |    sum {
        |      float
        |      int
        |      dec
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.pathAsDouble("data.aggregateItem.sum.float") should be(1.5)
    result.pathAsInt("data.aggregateItem.sum.int") should be(3)
    result.pathAsString("data.aggregateItem.sum.dec") should be("1.5")

    result = server.query(
      """{
        |  aggregateItem(skip: 2) {
        |    sum {
        |      float
        |      int
        |      dec
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.pathAsDouble("data.aggregateItem.sum.float") should be(1.5)
    result.pathAsInt("data.aggregateItem.sum.int") should be(3)
    result.pathAsString("data.aggregateItem.sum.dec") should be("1.5")

    result = server.query(
      s"""{
        |  aggregateItem(cursor: { id: "3" }) {
        |    sum {
        |      float
        |      int
        |      dec
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.pathAsDouble("data.aggregateItem.sum.float") should be(1.5)
    result.pathAsInt("data.aggregateItem.sum.int") should be(3)
    result.pathAsString("data.aggregateItem.sum.dec") should be("1.5")
  }
}
