package queries.regressions

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorTag.SQLiteConnectorTag
import util._

// RS: Ported
class Prisma_1481Spec extends FlatSpec with Matchers with ApiSpecBase {
  override def runOnlyForConnectors: Set[ConnectorTag] = Set(SQLiteConnectorTag)

  // validates fix for
  // https://github.com/prisma/prisma-engines/issues/1481

  "executeRaw and updateMany in a transaction" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model User {
         |  id    String  @id @default(uuid())
         |  email String  @unique
         |  name  String?
         |}
       """.stripMargin
    }

    database.setup(project)

    val queries = Seq(
      """mutation {
        |  executeRaw(
        |    query: "UPDATE User SET name = ? WHERE id = ?;"
        |    parameters: "[\"blub1\", \"THIS_DOES_NOT_EXIST1\"]"
        |  )
        |}""".stripMargin,
      """mutation {
        |  updateManyUser(
        |    where: { name: "A" }
        |    data:  { name: "B" }
        |  ) {
        |    count
        |  }
        |}""".stripMargin,
    )

    server.batch(queries, transaction = true, project, legacy = false).toString() should be("""{"batchResult":[{"data":{"executeRaw":0}},{"data":{"updateManyUser":{"count":0}}}]}""")
  }
}
