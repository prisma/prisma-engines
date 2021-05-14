package queries.aggregation

import org.scalatest.{FlatSpec, Matchers}
import util._

class DecimalAggregationQuerySpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = SchemaDsl.fromStringV11() {
    """model Item {
      |  id    String  @id @default(cuid())
      |  dec   Decimal @map("db_dec")
      |}
    """.stripMargin
  }

  override protected def beforeEach(): Unit = {
    super.beforeEach()
    database.setup(project)
  }

  def createItem(dec: String, id: Option[String] = None) = {
    val idString = id match {
      case Some(i) => s"""id: "$i","""
      case None    => ""
    }

    server.query(
      s"""mutation {
         |  createItem(data: { $idString dec: "$dec" }) {
         |    id
         |  }
         |}""".stripMargin,
      project
    )
  }

  "Aggregating an empty database" should "return 0 for all aggregations" taggedAs IgnoreSQLite in {
    val result = server.query(
      s"""{
         |  aggregateItem {
         |    count { dec }
         |    sum { dec }
         |    avg { dec }
         |    min { dec }
         |    max { dec }
         |  }
         |}""".stripMargin,
      project
    )

    result.toString should be("""{"data":{"aggregateItem":{"count":{"dec":0},"sum":{"dec":null},"avg":{"dec":null},"min":{"dec":null},"max":{"dec":null}}}}""")
  }

  "Using a combination of aggregations with some records in the database" should "return the correct results for decimals" taggedAs IgnoreSQLite in {
    createItem("5.5")
    createItem("4.5")

    val result = server.query(
      s"""
         |{
         |  aggregateItem {
         |    count { dec }
         |    sum { dec }
         |    avg { dec }
         |    min { dec }
         |    max { dec }
         |  }
         |}
       """.stripMargin,
      project
    )

    result.toString should be("""{"data":{"aggregateItem":{"count":{"dec":2},"sum":{"dec":"10"},"avg":{"dec":"5"},"min":{"dec":"4.5"},"max":{"dec":"5.5"}}}}""")
  }

  "Using a combination of aggregations with all sorts of query arguments" should "work" taggedAs IgnoreSQLite in {
    createItem("5.5", Some("1"))
    createItem("4.5", Some("2"))
    createItem("1.5", Some("3"))
    createItem("0", Some("4"))

    var result = server.query(
      """
        |{
        |  aggregateItem(take: 2) {
        |    count { dec }
        |    sum { dec }
        |    avg { dec }
        |    min { dec }
        |    max { dec }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.toString should be("""{"data":{"aggregateItem":{"count":{"dec":2},"sum":{"dec":"10"},"avg":{"dec":"5"},"min":{"dec":"4.5"},"max":{"dec":"5.5"}}}}""")

    result = server.query(
      """{
        |  aggregateItem(take: 5) {
        |    count { dec }
        |    sum { dec }
        |    avg { dec }
        |    min { dec }
        |    max { dec }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"aggregateItem":{"count":{"dec":4},"sum":{"dec":"11.5"},"avg":{"dec":"2.875"},"min":{"dec":"0"},"max":{"dec":"5.5"}}}}""")

    result = server.query(
      """{
        |  aggregateItem(take: -5) {
        |    count { dec }
        |    sum { dec }
        |    avg { dec }
        |    min { dec }
        |    max { dec }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"aggregateItem":{"count":{"dec":4},"sum":{"dec":"11.5"},"avg":{"dec":"2.875"},"min":{"dec":"0"},"max":{"dec":"5.5"}}}}""")

    result = server.query(
      """{
        |  aggregateItem(where: { id: { gt: "2" }}) {
        |    count { dec }
        |    sum { dec }
        |    avg { dec }
        |    min { dec }
        |    max { dec }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.toString should be(
      s"""{"data":{"aggregateItem":{"count":{"dec":2},"sum":{"dec":"1.5"},"avg":{"dec":"0.75"},"min":{"dec":"0"},"max":{"dec":"1.5"}}}}""")

    result = server.query(
      """{
        |  aggregateItem(skip: 2) {
        |    count { dec }
        |    sum { dec }
        |    avg { dec }
        |    min { dec }
        |    max { dec }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.toString should be(
      s"""{"data":{"aggregateItem":{"count":{"dec":2},"sum":{"dec":"1.5"},"avg":{"dec":"0.75"},"min":{"dec":"0"},"max":{"dec":"1.5"}}}}""")

    result = server.query(
      s"""{
        |  aggregateItem(cursor: { id: "3" }) {
        |    count { dec }
        |    sum { dec }
        |    avg { dec }
        |    min { dec }
        |    max { dec }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.toString should be(
      s"""{"data":{"aggregateItem":{"count":{"dec":2},"sum":{"dec":"1.5"},"avg":{"dec":"0.75"},"min":{"dec":"0"},"max":{"dec":"1.5"}}}}""")
  }
}
