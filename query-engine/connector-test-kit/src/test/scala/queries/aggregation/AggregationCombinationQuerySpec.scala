package queries.aggregation

import org.scalatest.{FlatSpec, Matchers}
import play.api.libs.json.JsNull
import util._

class AggregationCombinationQuerySpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = SchemaDsl.fromStringV11() {
    """model Item {
      |  id    String  @id @default(cuid())
      |  float Float   @map("db_float")
      |  int   Int     @map("db_int")
      |  dec   Decimal @map("db_dec")
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

  "Using a combination of aggregations with no records in the database" should "return 0 for all aggregations" in {
    val result = server.query(
      s"""{
         |  aggregateItem {
         |    count
         |    sum {
         |      float
         |      int
         |      dec
         |    }
         |    avg {
         |      float
         |      int
         |      dec
         |    }
         |    min {
         |      float
         |      int
         |      dec
         |    }
         |    max {
         |      float
         |      int
         |      dec
         |    }
         |  }
         |}""".stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"aggregateItem":{"count":0,"sum":{"float":0,"int":0,"dec":"0"},"avg":{"float":0,"int":0,"dec":"0"},"min":{"float":0,"int":0,"dec":"0"},"max":{"float":0,"int":0,"dec":"0"}}}}""")
  }

  "Using a combination of aggregations with some records in the database" should "return the correct results for each aggregation" in {
    createItem(5.5, 5, "5.5")
    createItem(4.5, 10, "4.5")

    val result = server.query(
      s"""
         |{
         |  aggregateItem {
         |    count
         |    sum {
         |      float
         |      int
         |      dec
         |    }
         |    avg {
         |      float
         |      int
         |      dec
         |    }
         |    min {
         |      float
         |      int
         |      dec
         |    }
         |    max {
         |      float
         |      int
         |      dec
         |    }
         |  }
         |}
       """.stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"aggregateItem":{"count":2,"sum":{"float":10,"int":15,"dec":"10"},"avg":{"float":5,"int":7.5,"dec":"5"},"min":{"float":4.5,"int":5,"dec":"4.5"},"max":{"float":5.5,"int":10,"dec":"5.5"}}}}""")
  }

  "Using a combination of aggregations with all sorts of query arguments" should "work" in {
    createItem(5.5, 5, "5.5", Some("1"))
    createItem(4.5, 10, "4.5", Some("2"))
    createItem(1.5, 2, "1.5", Some("3"))
    createItem(0.0, 1, "0", Some("4"))

    var result = server.query(
      """
        |{
        |  aggregateItem(take: 2) {
        |    count
        |    sum {
        |      float
        |      int
        |      dec
        |    }
        |    avg {
        |      float
        |      int
        |      dec
        |    }
        |    min {
        |      float
        |      int
        |      dec
        |    }
        |    max {
        |      float
        |      int
        |      dec
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"aggregateItem":{"count":2,"sum":{"float":10,"int":15,"dec":"10"},"avg":{"float":5,"int":7.5,"dec":"5"},"min":{"float":4.5,"int":5,"dec":"4.5"},"max":{"float":5.5,"int":10,"dec":"5.5"}}}}""")

    result = server.query(
      """{
        |  aggregateItem(take: 5) {
        |    count
        |    sum {
        |      float
        |      int
        |      dec
        |    }
        |    avg {
        |      float
        |      int
        |      dec
        |    }
        |    min {
        |      float
        |      int
        |      dec
        |    }
        |    max {
        |      float
        |      int
        |      dec
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"aggregateItem":{"count":4,"sum":{"float":11.5,"int":18,"dec":"11.5"},"avg":{"float":2.875,"int":4.5,"dec":"2.875"},"min":{"float":0,"int":1,"dec":"0"},"max":{"float":5.5,"int":10,"dec":"5.5"}}}}""")

    result = server.query(
      """{
        |  aggregateItem(take: -5) {
        |    count
        |    sum {
        |      float
        |      int
        |      dec
        |    }
        |    avg {
        |      float
        |      int
        |      dec
        |    }
        |    min {
        |      float
        |      int
        |      dec
        |    }
        |    max {
        |      float
        |      int
        |      dec
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"aggregateItem":{"count":4,"sum":{"float":11.5,"int":18,"dec":"11.5"},"avg":{"float":2.875,"int":4.5,"dec":"2.875"},"min":{"float":0,"int":1,"dec":"0"},"max":{"float":5.5,"int":10,"dec":"5.5"}}}}""")

    result = server.query(
      """{
        |  aggregateItem(where: { id: { gt: "2" }}) {
        |    count
        |    sum {
        |      float
        |      int
        |      dec
        |    }
        |    avg {
        |      float
        |      int
        |      dec
        |    }
        |    min {
        |      float
        |      int
        |      dec
        |    }
        |    max {
        |      float
        |      int
        |      dec
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"aggregateItem":{"count":2,"sum":{"float":1.5,"int":3,"dec":"1.5"},"avg":{"float":0.75,"int":1.5,"dec":"0.75"},"min":{"float":0,"int":1,"dec":"0"},"max":{"float":1.5,"int":2,"dec":"1.5"}}}}""")

    result = server.query(
      """{
        |  aggregateItem(skip: 2) {
        |    count
        |    sum {
        |      float
        |      int
        |      dec
        |    }
        |    avg {
        |      float
        |      int
        |      dec
        |    }
        |    min {
        |      float
        |      int
        |      dec
        |    }
        |    max {
        |      float
        |      int
        |      dec
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"aggregateItem":{"count":2,"sum":{"float":1.5,"int":3,"dec":"1.5"},"avg":{"float":0.75,"int":1.5,"dec":"0.75"},"min":{"float":0,"int":1,"dec":"0"},"max":{"float":1.5,"int":2,"dec":"1.5"}}}}""")

    result = server.query(
      s"""{
        |  aggregateItem(cursor: { id: "3" }) {
        |    count
        |    sum {
        |      float
        |      int
        |      dec
        |    }
        |    avg {
        |      float
        |      int
        |      dec
        |    }
        |    min {
        |      float
        |      int
        |      dec
        |    }
        |    max {
        |      float
        |      int
        |      dec
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"aggregateItem":{"count":2,"sum":{"float":1.5,"int":3,"dec":"1.5"},"avg":{"float":0.75,"int":1.5,"dec":"0.75"},"min":{"float":0,"int":1,"dec":"0"},"max":{"float":1.5,"int":2,"dec":"1.5"}}}}""")
  }

  "Using any aggregation with an unstable cursor" should "fail" in {
    createItem(5.5, 5, "5.5", Some("1"))
    createItem(4.5, 10, "4.5", Some("2"))
    createItem(1.5, 2, "1.5", Some("3"))
    createItem(0.0, 1, "0", Some("4"))

    server.queryThatMustFail(
      s"""{
         |  aggregateItem(cursor: { id: "3" }, orderBy: { float: asc }) {
         |    count
         |  }
         |}
      """.stripMargin,
      project,
      errorCode = 2019,
      errorContains = "The chosen cursor and orderBy combination is not stable"
    )
  }
}
