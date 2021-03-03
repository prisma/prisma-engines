package queries.orderAndPagination

import org.scalatest.{FlatSpec, Matchers}
import util._

class OrderByAggregationSpec extends FlatSpec with Matchers with ApiSpecBase {

  val project = SchemaDsl.fromStringV11() {
    """
      |model User {
      |  id    Int    @id @default(autoincrement())
      |  name  String
      |  posts Post[]
      |}
      |
      |model Post {
      |  id     Int    @id @default(autoincrement())
      |  title  String
      |  user   User   @relation(fields: [userId], references: [id])
      |  userId Int
      |}
      """.stripMargin
  }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)
    createTestData()
  }

  "Ordering by m2m count asc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyUser(orderBy: { posts: { count: asc } }) {
        |    id
        |    posts {
        |      title
        |    }
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be(
      """{"data":{"findManyUser":[{"id":1,"posts":[{"title":"alice_post_1"}]},{"id":2,"posts":[{"title":"bob_post_1"},{"title":"bob_post_2"}]}]}}"""
    )
  }

  "Ordering by m2m count desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyUser(orderBy: { posts: { count: desc } }) {
        |    id
        |    posts {
        |      title
        |    }
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be(
      """{"data":{"findManyUser":[{"id":2,"posts":[{"title":"bob_post_1"},{"title":"bob_post_2"}]},{"id":1,"posts":[{"title":"alice_post_1"}]}]}}"""
    )
  }

  private def createTestData(): Unit = {
    server.query("""mutation {createOneUser(data: { name: "Alice", posts: { create:  { title: "alice_post_1" } } }){ id }}""".stripMargin, project, legacy = false)
    server.query("""mutation {createOneUser(data: { name: "Bob",   posts: { create: [{ title: "bob_post_1" }, { title: "bob_post_2" }] } }){ id }}""".stripMargin, project, legacy = false)
  }
}
