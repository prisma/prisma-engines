package queries.aggregation

import org.scalatest.{FlatSpec, Matchers}
import play.api.libs.json.JsNull
import util._

// RS: Ported
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
         |    _count { _all }
         |    _sum {
         |      float
         |      int
         |      dec
         |    }
         |    _avg {
         |      float
         |      int
         |      dec
         |    }
         |    _min {
         |      float
         |      int
         |      dec
         |    }
         |    _max {
         |      float
         |      int
         |      dec
         |    }
         |  }
         |}""".stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"aggregateItem":{"_count":{"_all":0},"_sum":{"float":null,"int":null,"dec":null},"_avg":{"float":null,"int":null,"dec":null},"_min":{"float":null,"int":null,"dec":null},"_max":{"float":null,"int":null,"dec":null}}}}""")
  }

  "Using a combination of aggregations with some records in the database" should "return the correct results for each aggregation" in {
    createItem(5.5, 5, "5.5")
    createItem(4.5, 10, "4.5")

    val result = server.query(
      s"""
         |{
         |  aggregateItem {
         |    _count { _all }
         |    _sum {
         |      float
         |      int
         |      dec
         |    }
         |    _avg {
         |      float
         |      int
         |      dec
         |    }
         |    _min {
         |      float
         |      int
         |      dec
         |    }
         |    _max {
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
      """{"data":{"aggregateItem":{"_count":{"_all":2},"_sum":{"float":10,"int":15,"dec":"10"},"_avg":{"float":5,"int":7.5,"dec":"5"},"_min":{"float":4.5,"int":5,"dec":"4.5"},"_max":{"float":5.5,"int":10,"dec":"5.5"}}}}""")
  }

  "Using a combination of aggregations with all sorts of query arguments" should "work" in {
    createItem(5.5, 5, "5.5", Some("1"))
    createItem(4.5, 10, "4.5", Some("2"))
    createItem(1.5, 2, "1.5", Some("3"))
    createItem(0, 1, "0", Some("4"))

    var result = server.query(
      """
        |{
        |  aggregateItem(take: 2) {
        |    _count { _all }
        |    _sum {
        |      float
        |      int
        |      dec
        |    }
        |    _avg {
        |      float
        |      int
        |      dec
        |    }
        |    _min {
        |      float
        |      int
        |      dec
        |    }
        |    _max {
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
      """{"data":{"aggregateItem":{"_count":{"_all":2},"_sum":{"float":10,"int":15,"dec":"10"},"_avg":{"float":5,"int":7.5,"dec":"5"},"_min":{"float":4.5,"int":5,"dec":"4.5"},"_max":{"float":5.5,"int":10,"dec":"5.5"}}}}""")

    result = server.query(
      """{
        |  aggregateItem(take: 5) {
        |    _count { _all }
        |    _sum {
        |      float
        |      int
        |      dec
        |    }
        |    _avg {
        |      float
        |      int
        |      dec
        |    }
        |    _min {
        |      float
        |      int
        |      dec
        |    }
        |    _max {
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
      """{"data":{"aggregateItem":{"_count":{"_all":4},"_sum":{"float":11.5,"int":18,"dec":"11.5"},"_avg":{"float":2.875,"int":4.5,"dec":"2.875"},"_min":{"float":0,"int":1,"dec":"0"},"_max":{"float":5.5,"int":10,"dec":"5.5"}}}}""")

    result = server.query(
      """{
        |  aggregateItem(take: -5) {
        |    _count { _all }
        |    _sum {
        |      float
        |      int
        |      dec
        |    }
        |    _avg {
        |      float
        |      int
        |      dec
        |    }
        |    _min {
        |      float
        |      int
        |      dec
        |    }
        |    _max {
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
      """{"data":{"aggregateItem":{"_count":{"_all":4},"_sum":{"float":11.5,"int":18,"dec":"11.5"},"_avg":{"float":2.875,"int":4.5,"dec":"2.875"},"_min":{"float":0,"int":1,"dec":"0"},"_max":{"float":5.5,"int":10,"dec":"5.5"}}}}""")

    result = server.query(
      """{
        |  aggregateItem(where: { id: { gt: "2" }}) {
        |    _count { _all }
        |    _sum {
        |      float
        |      int
        |      dec
        |    }
        |    _avg {
        |      float
        |      int
        |      dec
        |    }
        |    _min {
        |      float
        |      int
        |      dec
        |    }
        |    _max {
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
      s"""{"data":{"aggregateItem":{"_count":{"_all":2},"_sum":{"float":1.5,"int":3,"dec":"1.5"},"_avg":{"float":0.75,"int":1.5,"dec":"0.75"},"_min":{"float":0,"int":1,"dec":"0"},"_max":{"float":1.5,"int":2,"dec":"1.5"}}}}""")

    result = server.query(
      """{
        |  aggregateItem(skip: 2) {
        |    _count { _all }
        |    _sum {
        |      float
        |      int
        |      dec
        |    }
        |    _avg {
        |      float
        |      int
        |      dec
        |    }
        |    _min {
        |      float
        |      int
        |      dec
        |    }
        |    _max {
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
      s"""{"data":{"aggregateItem":{"_count":{"_all":2},"_sum":{"float":1.5,"int":3,"dec":"1.5"},"_avg":{"float":0.75,"int":1.5,"dec":"0.75"},"_min":{"float":0,"int":1,"dec":"0"},"_max":{"float":1.5,"int":2,"dec":"1.5"}}}}""")

    result = server.query(
      s"""{
        |  aggregateItem(cursor: { id: "3" }) {
        |    _count { _all }
        |    _sum {
        |      float
        |      int
        |      dec
        |    }
        |    _avg {
        |      float
        |      int
        |      dec
        |    }
        |    _min {
        |      float
        |      int
        |      dec
        |    }
        |    _max {
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
      s"""{"data":{"aggregateItem":{"_count":{"_all":2},"_sum":{"float":1.5,"int":3,"dec":"1.5"},"_avg":{"float":0.75,"int":1.5,"dec":"0.75"},"_min":{"float":0,"int":1,"dec":"0"},"_max":{"float":1.5,"int":2,"dec":"1.5"}}}}""")
  }

  "Using any aggregation with an unstable cursor" should "fail" in {
    createItem(5.5, 5, "5.5", Some("1"))
    createItem(4.5, 10, "4.5", Some("2"))
    createItem(1.5, 2, "1.5", Some("3"))
    createItem(0, 1, "0", Some("4"))

    server.queryThatMustFail(
      s"""{
         |  aggregateItem(cursor: { id: "3" }, orderBy: { float: asc }) {
         |    _count { _all }
         |  }
         |}
      """.stripMargin,
      project,
      errorCode = 2019,
      errorContains = "Unable to process combination of query arguments for aggregation query"
    )
  }
}
