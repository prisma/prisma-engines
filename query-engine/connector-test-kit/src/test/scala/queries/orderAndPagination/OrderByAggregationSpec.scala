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
      |  categories Category[]
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
      |  users  User[]
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
      """{"data":{"findManyUser":[{"id":2,"posts":[{"title":"bob_post_1"},{"title":"bob_post_2"}]},{"id":1,"posts":[{"title":"alice_post_1"}]}]}}"""
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

  "[Combo] Ordering by one2m count asc + field asc" should "work" in {
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

  "[Combo] Ordering by one2m count asc + field desc" should "work" in {
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

  "[Combo] Ordering by m2m count asc + field desc" should "work" in {
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

  "[Combo] Ordering by one2m field asc + m2m count desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: [{ user: { name: asc }}, { categories: { count: desc }}]) {
        |    title
        |    user {
        |      name
        |    }
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

  "[2+ Hops] Ordering by m2one2m count asc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: { user: { categories: { count: asc } } }) {
        |    id
        |    user { categories { name } }
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be(
      """{"data":{"findManyPost":[{"id":1,"user":{"categories":[{"name":"Startup"}]}},{"id":2,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":3,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}}]}}"""
    )
  }

  "[2+ Hops] Ordering by m2one2m count desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: { user: { categories: { count: desc } } }) {
        |    id
        |    user { categories { name } }
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be(
      """{"data":{"findManyPost":[{"id":2,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":3,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":1,"user":{"categories":[{"name":"Startup"}]}}]}}"""
    )
  }

  "[Combo][2+ Hops] Ordering by m2m count asc + m2one2m count desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: [{ categories: { count: asc }}, { user: { categories: { count: desc }} }]) {
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
      """"data":{"findManyPost":[{"title":"bob_post_1","categories":[{"name":"Finance"}]},{"title":"bob_post_2","categories":[{"name":"History"}]},{"title":"alice_post_1","categories":[{"name":"News"},{"name":"Society"}]}]}}"""
    )
  }

  "[Combo][2+ Hops] Ordering by m2one field asc + m2one2m count desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: [{ user: { name: asc }}, { user: { categories: { count: desc }} }]) {
        |    id
        |    user {
        |      name
        |      categories { name }
        |    }
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be(
      """"{"data":{"findManyPost":[{"id":1,"user":{"name":"Alice","categories":[{"name":"Startup"}]}},{"id":2,"user":{"name":"Bob","categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":3,"user":{"name":"Bob","categories":[{"name":"Computer Science"},{"name":"Music"}]}}]}}"""
    )
  }

  // With pagination tests

  "[Cursor] Ordering by one2m count asc" should "work" in {
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

  "[Cursor] Ordering by one2m count desc" should "work" in {
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

  "[Cursor] Ordering by m2m count asc" should "work" in {
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

  "[Cursor] Ordering by m2m count desc" should "work" in {
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

  "[Cursor][Combo] Ordering by one2m count asc + field asc" should "work" in {
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

  "[Cursor][Combo] Ordering by one2m count asc + field desc" should "work" in {
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

  "[Cursor][Combo] Ordering by m2m count asc + field desc" should "work" in {
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

  "[Cursor][Combo] Ordering by one2m field asc + m2m count desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: [{ user: { name: asc }}, { categories: { count: desc }}], cursor: { id: 2 }, take: 2) {
        |    id
        |    title
        |    user {
        |      name
        |    }
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
      """{"data":{"findManyPost":[{"id":2,"title":"bob_post_1","user":{"name":"Bob"},"categories":[{"name":"Finance"}]},{"id":3,"title":"bob_post_2","user":{"name":"Bob"},"categories":[{"name":"History"}]}]}}"""
    )
  }

  "[Cursor][2+ Hops] Ordering by m2one2m count asc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: { user: { categories: { count: asc } } }, cursor: { id: 2 }, take: 1) {
        |    id
        |    user { categories { name } }
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be(
      """{"data":{"findManyPost":[{"id":2,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}}]}}"""
    )
  }

  "[Cursor][2+ Hops] Ordering by m2one2m count desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: { user: { categories: { count: desc } } }, cursor: { id: 2 }, take: 2) {
        |    id
        |    user { categories { name } }
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be(
      """{"data":{"findManyPost":[{"id":2,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":3,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}}]}}"""
    )
  }

  "[Cursor][Combo][2+ Hops] Ordering by m2m count asc + m2one2m count desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: [{ categories: { count: asc }}, { user: { categories: { count: desc }} }], cursor: { id: 3 }, take: 2) {
        |    id
        |    categories { name }
        |    user { categories { name } }
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be(
      """"{"data":{"findManyPost":[{"id":2,"categories":[{"name":"Finance"}],"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":3,"categories":[{"name":"History"}],"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}}]}}"""
    )
  }

  "[Cursor][Combo][2+ Hops] Ordering by m2one field asc + m2one2m count desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: [{ user: { name: asc }}, { user: { categories: { count: desc }} }], cursor: { id: 2 }, take: 2) {
        |    id
        |    user {
        |      name
        |      categories { name }
        |    }
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be(
      """"{"data":{"findManyPost":[{"id":2,"user":{"name":"Bob","categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":3,"user":{"name":"Bob","categories":[{"name":"Computer Science"},{"name":"Music"}]}}]}}"""
    )
  }

  private def createTestData(): Unit = {
    server.query("""mutation { createOneUser(data: { name: "Alice", categories: { create: [{ name: "Startup" }] }, posts: { create: { title: "alice_post_1", categories: { create: [{ name: "News" }, { name: "Society" }] }} } }){ id }}""".stripMargin, project, legacy = false)
    server.query("""mutation { createOneUser(data: { name: "Bob", categories: { create: [{ name: "Computer Science" }, { name: "Music" }] }, posts: { create: [{ title: "bob_post_1", categories: { create: [{ name: "Finance" }] } }, { title: "bob_post_2", categories: { create: [{ name: "History" }] } }] } }){ id }}""".stripMargin, project, legacy = false)
  }
}
