package queries.orderAndPagination

import org.scalatest.{FlatSpec, Matchers}
import util._

// RS: Ported
class OrderByAggregationSpec extends FlatSpec with Matchers with ApiSpecBase {

  val project = SchemaDsl.fromStringV11() {
    """
      |model User {
      |  id    Int    @id
      |  name  String
      |  posts Post[]
      |  categories Category[]
      |}
      |
      |model Post {
      |  id     Int    @id
      |  title  String
      |  user   User   @relation(fields: [userId], references: [id])
      |  userId Int
      |  categories Category[]
      |}
      |
      |model Category {
      |  id     Int    @id
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
        |  findManyUser(orderBy: { posts: { _count: asc } }) {
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
      """{"data":{"findManyUser":[{"id":3,"posts":[]},{"id":1,"posts":[{"title":"alice_post_1"}]},{"id":2,"posts":[{"title":"bob_post_1"},{"title":"bob_post_2"}]}]}}"""
    )
  }

  "Ordering by one2m count desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyUser(orderBy: { posts: { _count: desc } }) {
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
      """{"data":{"findManyUser":[{"id":2,"posts":[{"title":"bob_post_1"},{"title":"bob_post_2"}]},{"id":1,"posts":[{"title":"alice_post_1"}]},{"id":3,"posts":[]}]}}"""
    )
  }

  "Ordering by m2m count asc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: { categories: { _count: asc } }) {
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
      """{"data":{"findManyPost":[{"title":"bob_post_1","categories":[{"name":"Finance"}]},{"title":"alice_post_1","categories":[{"name":"News"},{"name":"Society"}]},{"title":"bob_post_2","categories":[{"name":"History"},{"name":"Gaming"},{"name":"Hacking"}]}]}}"""
    )
  }

  "Ordering by m2m count desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: { categories: { _count: desc } }) {
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
      """{"data":{"findManyPost":[{"title":"bob_post_2","categories":[{"name":"History"},{"name":"Gaming"},{"name":"Hacking"}]},{"title":"alice_post_1","categories":[{"name":"News"},{"name":"Society"}]},{"title":"bob_post_1","categories":[{"name":"Finance"}]}]}}"""
    )
  }

  "[Combo] Ordering by one2m count asc + field asc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyUser(orderBy: [{ posts: { _count: asc } }, { name: asc }]) {
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
      """{"data":{"findManyUser":[{"id":3,"name":"Motongo","posts":[]},{"id":1,"name":"Alice","posts":[{"title":"alice_post_1"}]},{"id":2,"name":"Bob","posts":[{"title":"bob_post_1"},{"title":"bob_post_2"}]}]}}"""
    )
  }

  "[Combo] Ordering by one2m count asc + field desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyUser(orderBy: [{ name: desc }, { posts: { _count: asc } }]) {
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
      """{"data":{"findManyUser":[{"id":3,"name":"Motongo","posts":[]},{"id":2,"name":"Bob","posts":[{"title":"bob_post_1"},{"title":"bob_post_2"}]},{"id":1,"name":"Alice","posts":[{"title":"alice_post_1"}]}]}}"""
    )
  }

  "[Combo] Ordering by m2m count asc + field desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: [{ categories: { _count: asc } }, { title: asc }]) {
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
      """{"data":{"findManyPost":[{"title":"bob_post_1","categories":[{"name":"Finance"}]},{"title":"alice_post_1","categories":[{"name":"News"},{"name":"Society"}]},{"title":"bob_post_2","categories":[{"name":"History"},{"name":"Gaming"},{"name":"Hacking"}]}]}}"""
    )
  }

  "[Combo] Ordering by one2m field asc + m2m count desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: [{ user: { name: asc }}, { categories: { _count: desc }}]) {
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
      """{"data":{"findManyPost":[{"user":{"name":"Alice"},"categories":[{"name":"News"},{"name":"Society"}]},{"user":{"name":"Bob"},"categories":[{"name":"History"},{"name":"Gaming"},{"name":"Hacking"}]},{"user":{"name":"Bob"},"categories":[{"name":"Finance"}]}]}}"""
    )
  }

  "[2+ Hops] Ordering by m2one2m count asc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: [{ user: { categories: { _count: asc } } }, { id: asc }]) {
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
        |  findManyPost(orderBy: { user: { categories: { _count: desc } } }) {
        |    id
        |    user { categories { name } }
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    val possibleResults = Set(
      """{"data":{"findManyPost":[{"id":2,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":3,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":1,"user":{"categories":[{"name":"Startup"}]}}]}}""",
      """{"data":{"findManyPost":[{"id":3,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":2,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":1,"user":{"categories":[{"name":"Startup"}]}}]}}"""
    )

    possibleResults should contain(result.toString)
  }

  "[Combo][2+ Hops] Ordering by m2m count asc + m2one2m count desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: [{ categories: { _count: asc }}, { user: { categories: { _count: desc }} }]) {
        |    id
        |    categories(orderBy: { name: asc }) {
        |      name
        |    }
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be(
      """{"data":{"findManyPost":[{"id":2,"categories":[{"name":"Finance"}]},{"id":1,"categories":[{"name":"News"},{"name":"Society"}]},{"id":3,"categories":[{"name":"Gaming"},{"name":"Hacking"},{"name":"History"}]}]}}"""
    )
  }

  "[Combo][2+ Hops] Ordering by m2one field asc + m2one2m count desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: [{ user: { name: asc }}, { user: { categories: { _count: desc }} }]) {
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

    val possibleResults = Set(
      """{"data":{"findManyPost":[{"id":1,"user":{"name":"Alice","categories":[{"name":"Startup"}]}},{"id":2,"user":{"name":"Bob","categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":3,"user":{"name":"Bob","categories":[{"name":"Computer Science"},{"name":"Music"}]}}]}}""",
      """{"data":{"findManyPost":[{"id":1,"user":{"name":"Alice","categories":[{"name":"Startup"}]}},{"id":3,"user":{"name":"Bob","categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":2,"user":{"name":"Bob","categories":[{"name":"Computer Science"},{"name":"Music"}]}}]}}"""
    )

    possibleResults should contain(result.toString)
  }

  // With pagination tests

  "[Cursor] Ordering by one2m count asc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyUser(orderBy: { posts: { _count: asc } }, cursor: { id: 2 }) {
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
        |  findManyUser(orderBy: { posts: { _count: desc } }, cursor: { id: 1 }) {
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
        |  findManyPost(orderBy: { categories: { _count: asc } }, cursor: { id: 2 }, take: 2) {
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
      """{"data":{"findManyPost":[{"id":2,"title":"bob_post_1","categories":[{"name":"Finance"}]},{"id":1,"title":"alice_post_1","categories":[{"name":"News"},{"name":"Society"}]}]}}"""
    )
  }

  "[Cursor] Ordering by m2m count desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: { categories: { _count: desc } }, cursor: { id: 1 }, take: 2) {
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
        |  findManyUser(orderBy: [{ posts: { _count: asc } }, { name: asc }], cursor: { id: 2 }) {
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
        |  findManyUser(orderBy: [{ name: desc }, { posts: { _count: asc } }], cursor: { id: 2 }, take: 1) {
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
        |  findManyPost(orderBy: [{ categories: { _count: asc } }, { title: asc }], cursor: { id: 2 }, take: 2) {
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
      """{"data":{"findManyPost":[{"id":2,"title":"bob_post_1","categories":[{"name":"Finance"}]},{"id":1,"title":"alice_post_1","categories":[{"name":"News"},{"name":"Society"}]}]}}"""
    )
  }

  "[Cursor][Combo] Ordering by one2m field asc + m2m count desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: [{ user: { name: asc }}, { categories: { _count: desc }}], cursor: { id: 1 }, take: 2) {
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
      """{"data":{"findManyPost":[{"id":1,"title":"alice_post_1","user":{"name":"Alice"},"categories":[{"name":"News"},{"name":"Society"}]},{"id":3,"title":"bob_post_2","user":{"name":"Bob"},"categories":[{"name":"History"},{"name":"Gaming"},{"name":"Hacking"}]}]}}"""
    )
  }

  "[Cursor][2+ Hops] Ordering by m2one2m count asc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: [{ user: { categories: { _count: asc } } }, { id: asc }], cursor: { id: 2 }, take: 1) {
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
        |  findManyPost(orderBy: [{ user: { categories: { _count: desc } } }, { id: asc }], cursor: { id: 2 }, take: 2) {
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
        |  findManyPost(orderBy: [{ categories: { _count: asc }}, { user: { categories: { _count: desc }} }], cursor: { id: 2 }, take: 2) {
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
      """{"data":{"findManyPost":[{"id":2,"categories":[{"name":"Finance"}],"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":1,"categories":[{"name":"News"},{"name":"Society"}],"user":{"categories":[{"name":"Startup"}]}}]}}"""
    )
  }

  "[Cursor][Combo][2+ Hops] Ordering by m2one field asc + m2one2m count desc" should "work" in {
    val result = server.query(
      """
        |{
        |  findManyPost(orderBy: [{ user: { name: asc }}, { user: { categories: { _count: desc }} }, { id: asc }], cursor: { id: 2 }, take: 2) {
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
      """{"data":{"findManyPost":[{"id":2,"user":{"name":"Bob","categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":3,"user":{"name":"Bob","categories":[{"name":"Computer Science"},{"name":"Music"}]}}]}}"""
    )
  }

  private def createTestData(): Unit = {
    server.query("""mutation { createOneUser(data: { id: 1, name: "Alice", categories: { create: [{ id: 1, name: "Startup" }] }, posts: { create: { id: 1, title: "alice_post_1", categories: { create: [{ id: 2, name: "News" }, { id: 3, name: "Society" }] }} } }){ id }}""".stripMargin, project, legacy = false)
    server.query("""mutation { createOneUser(data: { id: 2, name: "Bob", categories: { create: [{ id: 4, name: "Computer Science" }, { id: 5, name: "Music" }] }, posts: { create: [{ id: 2, title: "bob_post_1", categories: { create: [{ id: 6, name: "Finance" }] } }, { id: 3, title: "bob_post_2", categories: { create: [{ id: 7, name: "History" }, { id: 8, name: "Gaming" }, { id: 9, name: "Hacking" }] } }] } }){ id }}""".stripMargin, project, legacy = false)
    server.query("""mutation { createOneUser(data: { id: 3, name: "Motongo" }){ id }}""".stripMargin, project, legacy = false)
  }
}
