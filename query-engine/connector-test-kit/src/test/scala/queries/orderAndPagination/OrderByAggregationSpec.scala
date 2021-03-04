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
      |  categories Category[]
      |}
      |
      |model Category {
      |  id     Int    @id @default(autoincrement())
      |  name   String
      |  posts  Post[]
      |}
      """.stripMargin
  }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)
    createTestData()
  }

  "Ordering by one2m count asc" should "work" in {
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

  "[Multiple] Ordering by one2m count asc + simple order asc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyUser(orderBy: [{ posts: { count: asc } }, { name: asc }]) {
        |    id
        |    name
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
      """{"data":{"findManyUser":[{"id":1,"name":"Alice","posts":[{"title":"alice_post_1"}]},{"id":2,"name":"Bob","posts":[{"title":"bob_post_1"},{"title":"bob_post_2"}]}]}}"""
    )
  }

  "Ordering by one2m count desc" should "work" in {
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

  "[Multiple] Ordering by one2m count asc + simple order desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyUser(orderBy: [{ name: desc }, { posts: { count: asc } }]) {
        |    id
        |    name
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
      """{"data":{"findManyUser":[{"id":2,"name":"Bob","posts":[{"title":"bob_post_1"},{"title":"bob_post_2"}]},{"id":1,"name":"Alice","posts":[{"title":"alice_post_1"}]}]}}"""
    )
  }

  "Ordering by m2m count asc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: { categories: { count: asc } }) {
        |    title
        |    categories {
        |      name
        |    }
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be(
      """{"data":{"findManyPost":[{"title":"bob_post_1","categories":[{"name":"Finance"}]},{"title":"bob_post_2","categories":[{"name":"History"}]},{"title":"alice_post_1","categories":[{"name":"News"},{"name":"Society"}]}]}}"""
    )
  }

  "Ordering by m2m count desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: { categories: { count: desc } }) {
        |    title
        |    categories {
        |      name
        |    }
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be(
      """{"data":{"findManyPost":[{"title":"alice_post_1","categories":[{"name":"News"},{"name":"Society"}]},{"title":"bob_post_1","categories":[{"name":"Finance"}]},{"title":"bob_post_2","categories":[{"name":"History"}]}]}}"""
    )
  }

  "[Multiple] Ordering by m2m count asc + simple order desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: [{ categories: { count: asc } }, { title: asc }]) {
        |    title
        |    categories {
        |      name
        |    }
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be(
      """{"data":{"findManyPost":[{"title":"bob_post_1","categories":[{"name":"Finance"}]},{"title":"bob_post_2","categories":[{"name":"History"}]},{"title":"alice_post_1","categories":[{"name":"News"},{"name":"Society"}]}]}}"""
    )
  }

  // With pagination tests

  "[With Pagination] Ordering by one2m count asc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyUser(orderBy: { posts: { count: asc } }, cursor: { id: 2 }) {
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
      """{"data":{"findManyUser":[{"id":2,"posts":[{"title":"bob_post_1"},{"title":"bob_post_2"}]}]}}"""
    )
  }

  "[Multiple][With Pagination] Ordering by one2m count asc + simple order asc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyUser(orderBy: [{ posts: { count: asc } }, { name: asc }], cursor: { id: 2 }) {
        |    id
        |    name
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
      """{"data":{"findManyUser":[{"id":2,"name":"Bob","posts":[{"title":"bob_post_1"},{"title":"bob_post_2"}]}]}}"""
    )
  }

  "[With Pagination] Ordering by one2m count desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyUser(orderBy: { posts: { count: desc } }, cursor: { id: 1 }) {
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
      """{"data":{"findManyUser":[{"id":1,"posts":[{"title":"alice_post_1"}]}]}}"""
    )
  }

  "[Multiple][With Pagination] Ordering by one2m count asc + simple order desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyUser(orderBy: [{ name: desc }, { posts: { count: asc } }], cursor: { id: 2 }, take: 1) {
        |    id
        |    name
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
      """{"data":{"findManyUser":[{"id":2,"name":"Bob","posts":[{"title":"bob_post_1"},{"title":"bob_post_2"}]}]}}"""
    )
  }

  "[With Pagination] Ordering by m2m count asc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: { categories: { count: asc } }, cursor: { id: 2 }, take: 2) {
        |    id
        |    title
        |    categories {
        |      name
        |    }
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be(
      """{"data":{"findManyPost":[{"id":2,"title":"bob_post_1","categories":[{"name":"Finance"}]},{"id":3,"title":"bob_post_2","categories":[{"name":"History"}]}]}}"""
    )
  }

  "[With Pagination] Ordering by m2m count desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: { categories: { count: desc } }, cursor: { id: 1 }, take: 2) {
        |    id
        |    title
        |    categories {
        |      name
        |    }
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be(
      """{"data":{"findManyPost":[{"id":1,"title":"alice_post_1","categories":[{"name":"News"},{"name":"Society"}]},{"id":2,"title":"bob_post_1","categories":[{"name":"Finance"}]}]}}"""
    )
  }

  "[Multiple][With Pagination] Ordering by m2m count asc + simple order desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: [{ categories: { count: asc } }, { title: asc }], cursor: { id: 2 }, take: 2) {
        |    id
        |    title
        |    categories {
        |      name
        |    }
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be(
      """{"data":{"findManyPost":[{"id":2,"title":"bob_post_1","categories":[{"name":"Finance"}]},{"id":3,"title":"bob_post_2","categories":[{"name":"History"}]}]}}"""
    )
  }

  private def createTestData(): Unit = {
    server.query("""mutation { createOneUser(data: { name: "Alice", posts: { create: { title: "alice_post_1", categories: { create: [{ name: "News" }, { name: "Society" }] }} } }){ id }}""".stripMargin, project, legacy = false)
    server.query("""mutation { createOneUser(data: { name: "Bob", posts: { create: [{ title: "bob_post_1", categories: { create: [{ name: "Finance" }] } }, { title: "bob_post_2", categories: { create: [{ name: "History" }] } }] } }){ id }}""".stripMargin, project, legacy = false)
  }
}
