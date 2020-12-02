package queries.aggregation

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.Prisma2Capability
import util._

class CountAggregationQuerySpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = SchemaDsl.fromStringV11() {
    """model Item {
      |  id String @id @default(cuid())
      |  s1 String?
      |  s2 String?
      |}
    """.stripMargin
  }

  override protected def beforeEach(): Unit = {
    super.beforeEach()
    database.setup(project)
  }

  def createItem(s1: Option[String] = None, s2: Option[String] = None) = {
    def stringified(s: Option[String]) = s match {
      case Some(n) => s""""$n""""
      case None    => "null"
    }

    server.query(
      s"""mutation {
         |  createItem(data: { s1: ${stringified(s1)}, s2: ${stringified(s2)} }) {
         |    id
         |  }
         |}""".stripMargin,
      project
    )
  }

  "Counting with no records in the database" should "return 0" in {
    val result = server.query(
      s"""{
         |  aggregateItem {
             count { _all }
         |  }
         |}""".stripMargin,
      project
    )

    result.pathAsLong("data.aggregateItem.count._all") should be(0)
  }

  "Counting with 2 records in the database" should "return 2" in {
    createItem()
    createItem()

    val result = server.query(
      s"""{
         |  aggregateItem {
         |    count { _all }
         |  }
         |}""".stripMargin,
      project
    )

    result.pathAsLong("data.aggregateItem.count._all") should be(2)
  }

  "Counting fields that contain null" should "only return the count of these fields that don't have null" in {
    createItem(Some("1"), None)
    createItem(None, Some("1"))

    val result = server.query(
      s"""{
         |  aggregateItem {
         |    count {
         |      s1
         |      s2
         |    }
         |  }
         |}""".stripMargin,
      project
    )

    result.pathAsLong("data.aggregateItem.count.s1") should be(1)
    result.pathAsLong("data.aggregateItem.count.s2") should be(1)
  }

  "Counting with all sorts of query arguments" should "work" in {
    createItem(Some("1"))
    val i2 = createItem(Some("2"))
    createItem(Some("3"))
    createItem(Some("4"))

    val result = server.query(
      """{
        |  aggregateItem(take: 2) {
        |    count { _all }
        |  }
        |}
      """.stripMargin,
      project
    )

    result should equal("""{"data":{"aggregateItem":{"count":{"_all":2}}}}""".parseJson)

    val result2 = server.query(
      """{
        |  aggregateItem(take: 5) {
        |    count { _all }
        |  }
        |}
      """.stripMargin,
      project
    )

    result2 should equal("""{"data":{"aggregateItem":{"count":{"_all":4}}}}""".parseJson)

    val result3 = server.query(
      """{
        |  aggregateItem(take: -5) {
        |    count { _all }
        |  }
        |}
      """.stripMargin,
      project
    )

    result3 should equal("""{"data":{"aggregateItem":{"count":{"_all":4}}}}""".parseJson)

    val result4 = server.query(
      """{
        |  aggregateItem(where: { s1: { gt: "2" }}) {
        |    count { _all }
        |  }
        |}
      """.stripMargin,
      project
    )

    result4 should equal("""{"data":{"aggregateItem":{"count":{"_all":2}}}}""".parseJson)

    val result5 = server.query(
      """{
        |  aggregateItem(where: { s1: { gt: "1" }} orderBy: { s1: desc }) {
        |    count { _all }
        |  }
        |}
      """.stripMargin,
      project
    )

    result5 should equal("""{"data":{"aggregateItem":{"count":{"_all":3}}}}""".parseJson)

    val result6 = server.query(
      """{
        |  aggregateItem(skip: 2) {
        |    count { _all }
        |  }
        |}
      """.stripMargin,
      project
    )

    result6 should equal("""{"data":{"aggregateItem":{"count":{"_all":2}}}}""".parseJson)

    val result7 = server.query(
      s"""{
        |  aggregateItem(cursor: { id: "${i2.pathAsString("data.createItem.id")}" }) {
        |    count { _all }
        |  }
        |}
      """.stripMargin,
      project
    )

    result7 should equal("""{"data":{"aggregateItem":{"count":{"_all":3}}}}""".parseJson)
  }
}
