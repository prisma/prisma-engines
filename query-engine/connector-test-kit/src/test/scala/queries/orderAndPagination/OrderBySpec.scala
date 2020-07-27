package queries.orderAndPagination

import org.scalatest.{FlatSpec, Matchers}
import util._

class OrderBySpec extends FlatSpec with Matchers with ApiSpecBase {

  val project = SchemaDsl.fromStringV11() {
    """
      |model OrderTest {
      |  id            String @id
      |  uniqueField   Int    @unique
      |  nonUniqFieldA String
      |  nonUniqFieldB String
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
        |    uniqueFieldA
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
    val result = server.query(
      """
        |{
        |  findManyOrderTest(orderBy: { nonUniqFieldA: DESC }) {
        |    id
        |    nonUniqFieldA
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be(
      """{"data":{"findManyOrderTest":[{"id":"5","nonUniqField":"C"},{"id":"6","nonUniqField":"C"},{"id":"3","nonUniqField":"B"},{"id":"4","nonUniqField":"B"},{"id":"1","nonUniqField":"A"},{"id":"2","nonUniqField":"A"}]}}""")
  }

  "Ordering by multiple fields with at least one unique" should "work" in {
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
    server.query("""mutation {createOneOrderTest(data: { id: "1", uniqueField: 1, nonUniqFieldA: "A", nonUniqFieldB: "A"}){ id }}""", project, legacy = false)
    server.query("""mutation {createOneOrderTest(data: { id: "2", uniqueField: 2, nonUniqFieldA: "A", nonUniqFieldB: "B"}){ id }}""", project, legacy = false)
    server.query("""mutation {createOneOrderTest(data: { id: "3", uniqueField: 3, nonUniqFieldA: "B", nonUniqFieldB: "C"}){ id }}""", project, legacy = false)
    server.query("""mutation {createOneOrderTest(data: { id: "4", uniqueField: 4, nonUniqFieldA: "B", nonUniqFieldB: "A"}){ id }}""", project, legacy = false)
    server.query("""mutation {createOneOrderTest(data: { id: "5", uniqueField: 5, nonUniqFieldA: "C", nonUniqFieldB: "B"}){ id }}""", project, legacy = false)
    server.query("""mutation {createOneOrderTest(data: { id: "6", uniqueField: 6, nonUniqFieldA: "C", nonUniqFieldB: "C"}){ id }}""", project, legacy = false)
  }
}
