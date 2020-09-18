package queries.orderAndPagination

import org.scalatest.{FlatSpec, Matchers}
import util._

class OrderBySpec extends FlatSpec with Matchers with ApiSpecBase {

  val project = SchemaDsl.fromStringV11() {
    """
      |model OrderTest {
      |  uniqueField   Int    @unique
      |  nonUniqFieldA String
      |  nonUniqFieldB String
      |
      |  @@id([nonUniqFieldA, nonUniqFieldB])
      |}
    """
  }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)
    createTestData()
  }

  "Ordering by unique field ascending" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyOrderTest(orderBy: { uniqueField: asc }) {
        |    uniqueField
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be(
      """{"data":{"findManyOrderTest":[{"uniqueField":1},{"uniqueField":2},{"uniqueField":3},{"uniqueField":4},{"uniqueField":5},{"uniqueField":6}]}}""")
  }

  "Ordering by unique field descending" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyOrderTest(orderBy: { uniqueField: desc }) {
        |    uniqueField
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be(
      """{"data":{"findManyOrderTest":[{"uniqueField":6},{"uniqueField":5},{"uniqueField":4},{"uniqueField":3},{"uniqueField":2},{"uniqueField":1}]}}""")
  }

  "Ordering by multiple fields" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyOrderTest(orderBy: [{ nonUniqFieldA: desc }, { uniqueField: desc}]) {
        |    nonUniqFieldA
        |    uniqueField
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be(
      """{"data":{"findManyOrderTest":[{"nonUniqFieldA":"C","uniqueField":6},{"nonUniqFieldA":"C","uniqueField":5},{"nonUniqFieldA":"B","uniqueField":4},{"nonUniqFieldA":"B","uniqueField":3},{"nonUniqFieldA":"A","uniqueField":2},{"nonUniqFieldA":"A","uniqueField":1}]}}""")
  }

  "Ordering by multiple fields" should "honor the order of the ordering fields defined in the query" in {
    val result = server.query(
      """
        |{
        |  findManyOrderTest(orderBy: [{ nonUniqFieldB: asc }, { nonUniqFieldA: asc }, { uniqueField: asc}]) {
        |    nonUniqFieldB
        |    nonUniqFieldA
        |    uniqueField
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    // B ASC, A ASC, U ASC
    // A, A, 1
    // A, B, 4
    // B, A, 2
    // B, C, 5
    // C, B, 3
    // C, C, 6
    result.toString should be(
      """{"data":{"findManyOrderTest":[{"nonUniqFieldB":"A","nonUniqFieldA":"A","uniqueField":1},{"nonUniqFieldB":"A","nonUniqFieldA":"B","uniqueField":4},{"nonUniqFieldB":"B","nonUniqFieldA":"A","uniqueField":2},{"nonUniqFieldB":"B","nonUniqFieldA":"C","uniqueField":5},{"nonUniqFieldB":"C","nonUniqFieldA":"B","uniqueField":3},{"nonUniqFieldB":"C","nonUniqFieldA":"C","uniqueField":6}]}}""")
  }

  "Ordering with a negative take cursor" should "take from the end of the list" in {
    val result = server.query(
      """
        |{
        |  findManyOrderTest(take: -3, orderBy: { uniqueField: desc }) {
        |    uniqueField
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be("""{"data":{"findManyOrderTest":[{"uniqueField":3},{"uniqueField":2},{"uniqueField":1}]}}""")
  }

  "Ordering with empty objects" should "be allowed but have no changed behavior" in {
    val expected =
      """{"data":{"findManyOrderTest":[{"uniqueField":1},{"uniqueField":2},{"uniqueField":3},{"uniqueField":4},{"uniqueField":5},{"uniqueField":6}]}}"""

    val result1 = server.query(
      """
        |{
        |  findManyOrderTest(orderBy: {}) {
        |    uniqueField
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    val result2 = server.query(
      """
        |{
        |  findManyOrderTest(orderBy: [{}]) {
        |    uniqueField
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    val result3 = server.query(
      """
        |{
        |  findManyOrderTest(orderBy: [{}, {}]) {
        |    uniqueField
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result1.toString should be(expected)
    result2.toString should be(expected)
    result3.toString should be(expected)
  }

  private def createTestData(): Unit = {
    server.query("""mutation {createOneOrderTest(data: { uniqueField: 1, nonUniqFieldA: "A", nonUniqFieldB: "A"}){ uniqueField }}""", project, legacy = false)
    server.query("""mutation {createOneOrderTest(data: { uniqueField: 2, nonUniqFieldA: "A", nonUniqFieldB: "B"}){ uniqueField }}""", project, legacy = false)
    server.query("""mutation {createOneOrderTest(data: { uniqueField: 3, nonUniqFieldA: "B", nonUniqFieldB: "C"}){ uniqueField }}""", project, legacy = false)
    server.query("""mutation {createOneOrderTest(data: { uniqueField: 4, nonUniqFieldA: "B", nonUniqFieldB: "A"}){ uniqueField }}""", project, legacy = false)
    server.query("""mutation {createOneOrderTest(data: { uniqueField: 5, nonUniqFieldA: "C", nonUniqFieldB: "B"}){ uniqueField }}""", project, legacy = false)
    server.query("""mutation {createOneOrderTest(data: { uniqueField: 6, nonUniqFieldA: "C", nonUniqFieldB: "C"}){ uniqueField }}""", project, legacy = false)
  }
}
