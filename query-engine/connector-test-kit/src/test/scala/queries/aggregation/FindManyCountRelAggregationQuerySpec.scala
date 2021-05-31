package queries.aggregation

import org.scalatest.{FlatSpec, Matchers}
import util._

// RS: Ported
class FindManyCountRelAggregationQuerySpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = SchemaDsl.fromStringV11() {
    """
      |model Post {
      |  id         Int        @id @default(autoincrement())
      |  title      String
      |  comments   Comment[]
      |  categories Category[]
      |}
      |
      |model Comment {
      |  id      Int     @id @default(autoincrement())
      |  post    Post    @relation(fields: [postId], references: [id])
      |  postId  Int
      |}
      |
      |model Category {
      |  id    Int    @id @default(autoincrement())
      |  posts Post[]
      |}
      |
    """.stripMargin
  }

  override protected def beforeEach(): Unit = {
    super.beforeEach()
    database.setup(project)
  }

  def createPost(n_comments: Int, n_categories: Int) = {
    var renderedComments = Array.fill[Int](n_comments)(0).map(c => s"""{}""").mkString(",")
    var renderedCategories = Array.fill[Int](n_categories)(0).map(c => s"""{}""").mkString(",")

    if (n_comments == 0) {
      renderedComments = ""
    } else {
      renderedComments = s"""comments: { create: [$renderedComments] }"""
    }

    if (n_categories == 0) {
      renderedCategories = ""
    } else {
      renderedCategories = s"""categories: { create: [$renderedCategories] }"""
    }

    server.query(
      s"""mutation {
        |  createOnePost(
        |    data: {
        |      title: "a"
        |      $renderedComments
        |      $renderedCategories
        |    }
        |  ) {
        |    id
        |  }
        |}
        |""".stripMargin,
      project,
      legacy = false
    )
  }

  "Counting with no records in the database" should "return 0" in {
    createPost(0, 0)

    val res = server.query(
      s"""
         |query {
         |  findManyPost {
         |    _count { comments categories }
         |  }
         |}
         |""".stripMargin,
      project,
      legacy = false
    )

    res.toString() should be("""{"data":{"findManyPost":[{"_count":{"comments":0,"categories":0}}]}}""")
  }

  "Counting one2m and m2m records" should "work" in {
    createPost(1, 2)
    createPost(3, 4)

    val res = server.query(
      s"""
         |query {
         |  findManyPost(orderBy: { id: asc }) {
         |    _count { comments categories }
         |  }
         |}
         |""".stripMargin,
      project,
      legacy = false
    )

    res.toString() should be("""{"data":{"findManyPost":[{"_count":{"comments":1,"categories":2}},{"_count":{"comments":3,"categories":4}}]}}""")
  }

  "Counting with some records and filters" should "not affect the count" in {
    createPost(4, 4)

    val res = server.query(
      s"""
         |query {
         |  findManyPost(where: { id: 1 }) {
         |    comments(cursor: { id: 1 }, take: 1) { id }
         |    categories(cursor: { id: 1 }, take: 1) { id }
         |    _count { comments categories }
         |  }
         |}
         |""".stripMargin,
      project,
      legacy = false
    )

    res.toString() should be("""{"data":{"findManyPost":[{"comments":[{"id":1}],"categories":[{"id":1}],"_count":{"comments":4,"categories":4}}]}}""")
  }

}
