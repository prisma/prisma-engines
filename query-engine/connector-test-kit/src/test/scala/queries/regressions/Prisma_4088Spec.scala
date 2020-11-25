package queries.regressions

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class Regression4088Spec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  // Validates fix for: "Incorrect handling of "undefined" in queries"
  // https://github.com/prisma/prisma/issues/4088

  val project = ProjectDsl.fromString {
    """
      |model TestModel {
      |  id  String @id @default(cuid())
      |  str String
      |}
    """.stripMargin
  }

  def create(str: String, project: Project): String = {
    val res = server.query(
      s"""mutation {
         |  createOneTestModel(
         |    data: {
         |      str: "$str"
         |    })
         |  {
         |    id
         |  }
         |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.pathAsString("data.createOneTestModel.id")
  }

  "FindMany queries with an OR condition and one filter" should "only apply one filter" in {
    database.setup(project)
    create("aa", project)
    create("ab", project)
    create("ac", project)

    val res = server.query(
      """query {
        |  findManyTestModel(
        |    where: { OR: [{ str: { equals: "aa" } }]}
        |  ) {
        |    str
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be(s"""{\"data\":{\"findManyTestModel\":[{\"str\":\"aa\"}]}}""")
  }

  "FindMany queries with an OR condition and two filters, of which one is undefined" should "only apply one filter" in {
    database.setup(project)
    create("aa", project)
    create("ab", project)
    create("ac", project)

    val res = server.query(
      """query {
        |  findManyTestModel(
        |    where: { OR: [{ str: { equals: "aa" }}, {str: {} }]}
        |  ) {
        |    str
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be(s"""{\"data\":{\"findManyTestModel\":[{\"str\":\"aa\"}]}}""")
  }


  "FindMany queries with an OR condition and no filters" should "return an empty list" in {
    database.setup(project)
    create("aa", project)
    create("ab", project)
    create("ac", project)
    val res = server.query(
      """query {
        |  findManyTestModel(
        |    where: { OR: [] }
        |  ) {
        |    str
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be(s"""{\"data\":{\"findManyTestModel\":[]}}""")
  }

  "FindMany queries with an AND condition and no filters" should "return an empty list" in {
    database.setup(project)
    create("aa", project)
    create("ab", project)
    create("ac", project)
    val res = server.query(
      """query {
        |  findManyTestModel(
        |    where: { AND: [] }
        |  ) {
        |    str
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be(s"""{\"data\":{\"findManyTestModel\":[]}}""")
  }

  "FindMany queries with an AND condition and one filter" should "only apply one filter" in {
    database.setup(project)
    create("aa", project)
    create("ab", project)
    create("ac", project)

    val res = server.query(
      """query {
        |  findManyTestModel(
        |    where: { AND: [] }
        |  ) {
        |    str
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be(s"""{\"data\":{\"findManyTestModel\":[{\"str\":\"aa\"}]}}""")
  }

  "FindMany queries with an AND condition and two filters, of which one is undefined" should "only apply one filter" in {
    database.setup(project)
    create("aa", project)
    create("ab", project)
    create("ac", project)

    val res = server.query(
      """query {
        |  findManyTestModel(
        |    where: { AND: [{ str: { equals: "aa" }}, {str: {} }]}
        |  ) {
        |    str
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be(s"""{\"data\":{\"findManyTestModel\":[{\"str\":\"aa\"}]}}""")
  }

  "FindMany queries with an NOT condition and no filters" should "return all items" in {
    database.setup(project)
    create("aa", project)
    create("ab", project)
    create("ac", project)
    val res = server.query(
      """query {
        |  findManyTestModel(
        |    where: { NOT: [] }
        |  ) {
        |    str
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be(s"""{\"data\":{\"findManyTestModel\":[{\"str\":\"aa\"}, {\"str\":\"ab\"}, {\"str\":\"ac\"}]}}""")
  }

  "FindMany queries with an NOT condition and one filter " should "only apply one filter" in {
    database.setup(project)
    create("aa", project)
    create("ab", project)
    create("ac", project)

    val res = server.query(
      """query {
        |  findManyTestModel(
        |    where: { NOT: [] }
        |  ) {
        |    str
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be(s"""{\"data\":{\"findManyTestModel\":[{\"str\":\"aa\"}]}}""")
  }

  "FindMany queries with an NOT condition" should "and two filters, of which one is undefined, should only apply one filter" in {
    database.setup(project)
    create("aa", project)
    create("ab", project)
    create("ac", project)

    val res = server.query(
      """query {
        |  findManyTestModel(
        |    where: { NOT: [{ str: { equals: "aa" }}, {str: {} }]}
        |  ) {
        |    str
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be(s"""{\"data\":{\"findManyTestModel\":[{\"str\":\"aa\"}]}}""")
  }
}
