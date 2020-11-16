package queries.regressions

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class Regression4088Spec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  // Validates fix for: "Incorrect handling of "undefined" in queries"
  // https://github.com/prisma/prisma/issues/4088

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

  "FindMany queries" should "ignore undefined fields in OR queries" in {
    val project = ProjectDsl.fromString {
      """
        |model TestModel {
        |  id  String @id @default(cuid())
        |  str String
        |}
      """.stripMargin
    }
    database.setup(project)

    create("aa", project)
    create("ab", project)
    create("ac", project)

    server.queryThatMustFail(
      """query {
        |  findManyTestModel(
        |    where: { OR: [{ str: { equals: "aa" }}, {str: {} }]}
        |  ) {
        |    str
        |  }
        |}
      """.stripMargin,
      project,
      errorCode = 2009,
      errorContains = """A value is required but not set.""",
      legacy = false
    )
  }
}
