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
        |  findManyOrderTest(orderBy: { uniqueField: ASC }) {
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
        |  findManyOrderTest(orderBy: { uniqueField: DESC }) {
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

  "Ordering with on a non-unique field" should "implicitly order by ID" in {
    // Implicitly adds the second part of the primary key (nonUniqFieldB ASC) to the order_by.
    val result = server.query(
      """
        |{
        |  findManyOrderTest(orderBy: { nonUniqFieldA: DESC }) {
        |    nonUniqFieldA
        |    nonUniqFieldB
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be(
      """{"data":{"findManyOrderTest":[{"nonUniqFieldA":"C","nonUniqFieldB":"B"},{"nonUniqFieldA":"C","nonUniqFieldB":"C"},{"nonUniqFieldA":"B","nonUniqFieldB":"A"},{"nonUniqFieldA":"B","nonUniqFieldB":"C"},{"nonUniqFieldA":"A","nonUniqFieldB":"A"},{"nonUniqFieldA":"A","nonUniqFieldB":"B"}]}}""")
  }

  "Ordering by multiple fields with at least one individually unique field" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyOrderTest(orderBy: { nonUniqFieldA: DESC, uniqueField: DESC}) {
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
        |  findManyOrderTest(orderBy: { nonUniqFieldB: ASC, nonUniqFieldA: ASC, uniqueField: ASC}) {
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

  // -------------------------

  "Ordering with a negative take cursor" should "take the last 3 elements of the default order (ID ascending)" in {
    val result = server.query(
      """
        |{
        |  needsTiebreakers(take: -3) {
        |    order
        |  }
        |}
      """,
      project
    )

    result.toString should be("""{"data":{"needsTiebreakers":[{"order":5},{"order":6},{"order":7}]}}""")
  }

//  "The order when giving an order by ASC that only has ties" should "be by Id ascending and therefore oldest first" in {
//    val result = server.query(
//      """
//        |{
//        |  needsTiebreakers(orderBy: name_ASC) {
//        |    order
//        |  }
//        |}
//      """,
//      project
//    )
//
//    result.toString should be("""{"data":{"needsTiebreakers":[{"order":1},{"order":2},{"order":3},{"order":4},{"order":5},{"order":6},{"order":7}]}}""")
//  }
//
//  "The order when giving an order by ASC that is unique" should "be correct and the query should not include an ordering with the id tiebreaker" in {
//    val result = server.query(
//      """
//        |{
//        |  needsTiebreakers(orderBy: order_ASC) {
//        |    order
//        |  }
//        |}
//      """,
//      project
//    )
//
//    result.toString should be("""{"data":{"needsTiebreakers":[{"order":1},{"order":2},{"order":3},{"order":4},{"order":5},{"order":6},{"order":7}]}}""")
//  }
//
//  "The order when giving an order by ASC that only has ties and uses last" should "be by Id ascending and therefore oldest first" in {
//    val result = server.query(
//      """
//        |{
//        |  needsTiebreakers(orderBy: name_ASC, take: -3) {
//        |    order
//        |  }
//        |}
//      """,
//      project
//    )
//
//    result.toString should be("""{"data":{"needsTiebreakers":[{"order":5},{"order":6},{"order":7}]}}""")
//  }
//
//  "The order when giving an order by DESC that only has ties" should "be by Id ascending and therefore oldest first" in {
//    val result = server.query(
//      """
//        |{
//        |  needsTiebreakers(orderBy: name_DESC) {
//        |    order
//        |  }
//        |}
//      """,
//      project
//    )
//
//    result.toString should be("""{"data":{"needsTiebreakers":[{"order":1},{"order":2},{"order":3},{"order":4},{"order":5},{"order":6},{"order":7}]}}""")
//  }
//
//  "The order when giving an order by DESC that only has ties and uses last" should "be by Id ascending and therefore oldest first" in {
//    val result = server.query(
//      """
//        |{
//        |  needsTiebreakers(orderBy: name_DESC, take: -3) {
//        |    order
//        |  }
//        |}
//      """,
//      project
//    )
//
//    result.toString should be("""{"data":{"needsTiebreakers":[{"order":5},{"order":6},{"order":7}]}}""")
//  }

  private def createTestData(): Unit = {
    server.query("""mutation {createOneOrderTest(data: { uniqueField: 1, nonUniqFieldA: "A", nonUniqFieldB: "A"}){ uniqueField }}""", project, legacy = false)
    server.query("""mutation {createOneOrderTest(data: { uniqueField: 2, nonUniqFieldA: "A", nonUniqFieldB: "B"}){ uniqueField }}""", project, legacy = false)
    server.query("""mutation {createOneOrderTest(data: { uniqueField: 3, nonUniqFieldA: "B", nonUniqFieldB: "C"}){ uniqueField }}""", project, legacy = false)
    server.query("""mutation {createOneOrderTest(data: { uniqueField: 4, nonUniqFieldA: "B", nonUniqFieldB: "A"}){ uniqueField }}""", project, legacy = false)
    server.query("""mutation {createOneOrderTest(data: { uniqueField: 5, nonUniqFieldA: "C", nonUniqFieldB: "B"}){ uniqueField }}""", project, legacy = false)
    server.query("""mutation {createOneOrderTest(data: { uniqueField: 6, nonUniqFieldA: "C", nonUniqFieldB: "C"}){ uniqueField }}""", project, legacy = false)
  }
}
