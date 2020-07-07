package queries.aggregation

import org.scalatest.{FlatSpec, Matchers}
import play.api.libs.json.JsNull
import util._

class AggregationCombinationQuerySpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = SchemaDsl.fromStringV11() {
    """model Item {
      |  id    String @id @default(cuid())
      |  float Float
      |  int   Int
      |}
    """.stripMargin
  }

  override protected def beforeEach(): Unit = {
    super.beforeEach()
    database.setup(project)
  }

  def createItem(float: Double, int: Int, id: Option[String] = None) = {
    val idString = id match {
      case Some(i) => s"""id: "$i","""
      case None    => ""
    }

    server.query(
      s"""mutation {
         |  createItem(data: { $idString float: $float, int: $int }) {
         |    id
         |  }
         |}""".stripMargin,
      project
    )
  }

  "Using a combination of aggregations with no records in the database" should "return null for most aggregations" in {
    val result = server.query(
      s"""{
         |  aggregateItem {
         |    count
         |    sum {
         |      float
         |      int
         |    }
         |    avg {
         |      float
         |      int
         |    }
         |    min {
         |      float
         |      int
         |    }
         |    max {
         |      float
         |      int
         |    }
         |  }
         |}""".stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"aggregateItem":{"count":0,"sum":{"float":null,"int":null},"avg":{"float":null,"int":null},"min":{"float":null,"int":null},"max":{"float":null,"int":null}}}}""")
  }

  "Using a combination of aggregations with some records in the database" should "return the correct results for each aggregation" in {
    createItem(5.5, 5)
    createItem(4.5, 10)

    val result = server.query(
      s"""
         |{
         |  aggregateItem {
         |    count
         |    sum {
         |      float
         |      int
         |    }
         |    avg {
         |      float
         |      int
         |    }
         |    min {
         |      float
         |      int
         |    }
         |    max {
         |      float
         |      int
         |    }
         |  }
         |}
       """.stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"aggregateItem":{"count":2,"sum":{"float":10,"int":15},"avg":{"float":5,"int":7.5},"min":{"float":4.5,"int":5},"max":{"float":5.5,"int":10}}}}""")
  }

  "Using a combination of aggregations with all sorts of query arguments" should "work" in {
    createItem(5.5, 5, Some("1"))
    createItem(4.5, 10, Some("2"))
    createItem(1.5, 2, Some("3"))
    createItem(0.0, 1, Some("4"))

    var result = server.query(
      """
        |{
        |  aggregateItem(take: 2) {
        |    count
        |    sum {
        |      float
        |      int
        |    }
        |    avg {
        |      float
        |      int
        |    }
        |    min {
        |      float
        |      int
        |    }
        |    max {
        |      float
        |      int
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"aggregateItem":{"count":2,"sum":{"float":10,"int":15},"avg":{"float":5,"int":7.5},"min":{"float":4.5,"int":5},"max":{"float":5.5,"int":10}}}}""")

    result = server.query(
      """{
        |  aggregateItem(take: 5) {
        |    count
        |    sum {
        |      float
        |      int
        |    }
        |    avg {
        |      float
        |      int
        |    }
        |    min {
        |      float
        |      int
        |    }
        |    max {
        |      float
        |      int
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"aggregateItem":{"count":4,"sum":{"float":11.5,"int":18},"avg":{"float":2.875,"int":4.5},"min":{"float":0,"int":1},"max":{"float":5.5,"int":10}}}}""")

    result = server.query(
      """{
        |  aggregateItem(take: -5) {
        |    count
        |    sum {
        |      float
        |      int
        |    }
        |    avg {
        |      float
        |      int
        |    }
        |    min {
        |      float
        |      int
        |    }
        |    max {
        |      float
        |      int
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"aggregateItem":{"count":4,"sum":{"float":11.5,"int":18},"avg":{"float":2.875,"int":4.5},"min":{"float":0,"int":1},"max":{"float":5.5,"int":10}}}}""")

    result = server.query(
      """{
        |  aggregateItem(where: { id_gt: "2" }) {
        |    count
        |    sum {
        |      float
        |      int
        |    }
        |    avg {
        |      float
        |      int
        |    }
        |    min {
        |      float
        |      int
        |    }
        |    max {
        |      float
        |      int
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"aggregateItem":{"count":2,"sum":{"float":1.5,"int":3},"avg":{"float":0.75,"int":1.5},"min":{"float":0,"int":1},"max":{"float":1.5,"int":2}}}}""")

    result = server.query(
      """{
        |  aggregateItem(skip: 2) {
        |    count
        |    sum {
        |      float
        |      int
        |    }
        |    avg {
        |      float
        |      int
        |    }
        |    min {
        |      float
        |      int
        |    }
        |    max {
        |      float
        |      int
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"aggregateItem":{"count":2,"sum":{"float":1.5,"int":3},"avg":{"float":0.75,"int":1.5},"min":{"float":0,"int":1},"max":{"float":1.5,"int":2}}}}""")

    result = server.query(
      s"""{
        |  aggregateItem(cursor: { id: "3" }) {
        |    count
        |    sum {
        |      float
        |      int
        |    }
        |    avg {
        |      float
        |      int
        |    }
        |    min {
        |      float
        |      int
        |    }
        |    max {
        |      float
        |      int
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"aggregateItem":{"count":2,"sum":{"float":1.5,"int":3},"avg":{"float":0.75,"int":1.5},"min":{"float":0,"int":1},"max":{"float":1.5,"int":2}}}}""")
  }
}
