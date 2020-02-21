package queries.orderAndPagination

import org.scalatest.{FlatSpec, Matchers}
import util._

class OrderBySpec extends FlatSpec with Matchers with ApiSpecBase {

  val project = SchemaDsl.fromStringV11() {
    """
      |model NeedsTiebreaker {
      |  id    String @id @default(cuid())
      |  name  String
      |  order Int @unique
      |}
    """
  }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)
    createNeedsTiebreakers()
  }


   // we are no longer using id ordering by default
  "The order when not giving an order by" should "be by Id ascending and therefore oldest first" ignore {
    val resultWithOrderByImplicitlySpecified = server.query(
      """
        |{
        |  needsTiebreakers {
        |    order
        |  }
        |}
      """,
      project
    )

    resultWithOrderByImplicitlySpecified.toString should be(
      """{"data":{"needsTiebreakers":[{"order":1},{"order":2},{"order":3},{"order":4},{"order":5},{"order":6},{"order":7}]}}""")

    val resultWithOrderByExplicitlySpecified = server.query(
      """
        |{
        |  needsTiebreakers(orderBy: id_ASC) {
        |    order
        |  }
        |}
      """,
      project
    )
    resultWithOrderByImplicitlySpecified should be(resultWithOrderByExplicitlySpecified)
  }

  "The order when not giving an order by and using last" should "be by Id ascending and therefore oldest first" in {
    val result = server.query(
      """
        |{
        |  needsTiebreakers(last: 3) {
        |    order
        |  }
        |}
      """,
      project
    )

    result.toString should be("""{"data":{"needsTiebreakers":[{"order":5},{"order":6},{"order":7}]}}""")
  }

  "The order when giving an order by ASC that only has ties" should "be by Id ascending and therefore oldest first" in {
    val result = server.query(
      """
        |{
        |  needsTiebreakers(orderBy: name_ASC) {
        |    order
        |  }
        |}
      """,
      project
    )

    result.toString should be("""{"data":{"needsTiebreakers":[{"order":1},{"order":2},{"order":3},{"order":4},{"order":5},{"order":6},{"order":7}]}}""")
  }

  "The order when giving an order by ASC that is unique" should "be correct and the query should not include an ordering with the id tiebreaker" in {
    val result = server.query(
      """
        |{
        |  needsTiebreakers(orderBy: order_ASC) {
        |    order
        |  }
        |}
      """,
      project
    )

    result.toString should be("""{"data":{"needsTiebreakers":[{"order":1},{"order":2},{"order":3},{"order":4},{"order":5},{"order":6},{"order":7}]}}""")
  }

  "The order when giving an order by ASC that only has ties and uses last" should "be by Id ascending and therefore oldest first" in {
    val result = server.query(
      """
        |{
        |  needsTiebreakers(orderBy: name_ASC, last: 3) {
        |    order
        |  }
        |}
      """,
      project
    )

    result.toString should be("""{"data":{"needsTiebreakers":[{"order":5},{"order":6},{"order":7}]}}""")
  }

  "The order when giving an order by DESC that only has ties" should "be by Id ascending and therefore oldest first" in {
    val result = server.query(
      """
        |{
        |  needsTiebreakers(orderBy: name_DESC) {
        |    order
        |  }
        |}
      """,
      project
    )

    result.toString should be("""{"data":{"needsTiebreakers":[{"order":1},{"order":2},{"order":3},{"order":4},{"order":5},{"order":6},{"order":7}]}}""")
  }

  "The order when giving an order by DESC that only has ties and uses last" should "be by Id ascending and therefore oldest first" in {
    val result = server.query(
      """
        |{
        |  needsTiebreakers(orderBy: name_DESC, last: 3) {
        |    order
        |  }
        |}
      """,
      project
    )

    result.toString should be("""{"data":{"needsTiebreakers":[{"order":5},{"order":6},{"order":7}]}}""")
  }

  private def createNeedsTiebreakers(): Unit = {
    server.query("""mutation {createNeedsTiebreaker(data: {name: "SameSame", order: 1}){ id }}""", project)
    server.query("""mutation {createNeedsTiebreaker(data: {name: "SameSame", order: 2}){ id }}""", project)
    server.query("""mutation {createNeedsTiebreaker(data: {name: "SameSame", order: 3}){ id }}""", project)
    server.query("""mutation {createNeedsTiebreaker(data: {name: "SameSame", order: 4}){ id }}""", project)
    server.query("""mutation {createNeedsTiebreaker(data: {name: "SameSame", order: 5}){ id }}""", project)
    server.query("""mutation {createNeedsTiebreaker(data: {name: "SameSame", order: 6}){ id }}""", project)
    server.query("""mutation {createNeedsTiebreaker(data: {name: "SameSame", order: 7}){ id }}""", project)
  }
}
