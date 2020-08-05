package queries.aggregation

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.Prisma2Capability
import util._

class CountAggregationQuerySpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = SchemaDsl.fromStringV11() {
    """model Item {
      |  id   String @id @default(cuid())
      |  name String
      |}
    """.stripMargin
  }

  override protected def beforeEach(): Unit = {
    super.beforeEach()
    database.setup(project)
  }

  def createItem(name: String) = {
    server.query(
      s"""mutation {
         |  createItem(data: { name: "$name" }) {
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
         |    count
         |  }
         |}""".stripMargin,
      project
    )

    result.pathAsLong("data.aggregateItem.count") should be(0)
  }

  "Counting with 2 records in the database" should "return 2" in {
    createItem("1")
    createItem("2")

    val result = server.query(
      s"""{
         |  aggregateItem {
         |    count
         |  }
         |}""".stripMargin,
      project
    )

    result.pathAsLong("data.aggregateItem.count") should be(2)
  }

  "Counting with all sorts of query arguments" should "work" in {
    createItem("1")
    val i2 = createItem("2")
    createItem("3")
    createItem("4")

    val result = server.query(
      """{
        |  aggregateItem(take: 2) {
        |    count
        |  }
        |}
      """.stripMargin,
      project
    )

    result should equal("""{"data":{"aggregateItem":{"count":2}}}""".parseJson)

    val result2 = server.query(
      """{
        |  aggregateItem(take: 5) {
        |    count
        |  }
        |}
      """.stripMargin,
      project
    )

    result2 should equal("""{"data":{"aggregateItem":{"count":4}}}""".parseJson)

    val result3 = server.query(
      """{
        |  aggregateItem(take: -5) {
        |    count
        |  }
        |}
      """.stripMargin,
      project
    )

    result3 should equal("""{"data":{"aggregateItem":{"count":4}}}""".parseJson)

    val result4 = server.query(
      """{
        |  aggregateItem(where: { name_gt: "2" }) {
        |    count
        |  }
        |}
      """.stripMargin,
      project
    )

    result4 should equal("""{"data":{"aggregateItem":{"count":2}}}""".parseJson)

    val result5 = server.query(
      """{
        |  aggregateItem(where: { name_gt: "1" } orderBy: { name: desc }) {
        |    count
        |  }
        |}
      """.stripMargin,
      project
    )

    result5 should equal("""{"data":{"aggregateItem":{"count":3}}}""".parseJson)

    val result6 = server.query(
      """{
        |  aggregateItem(skip: 2) {
        |    count
        |  }
        |}
      """.stripMargin,
      project
    )

    result6 should equal("""{"data":{"aggregateItem":{"count":2}}}""".parseJson)

    val result7 = server.query(
      s"""{
        |  aggregateItem(cursor: { id: "${i2.pathAsString("data.createItem.id")}" }) {
        |    count
        |  }
        |}
      """.stripMargin,
      project
    )

    result7 should equal("""{"data":{"aggregateItem":{"count":3}}}""".parseJson)
  }
}
