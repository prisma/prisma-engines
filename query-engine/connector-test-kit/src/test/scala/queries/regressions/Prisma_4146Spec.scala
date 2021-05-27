package queries.regressions

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

// RS: Ported
class Prisma_4146Spec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  // validates fix for
  // https://github.com/prisma/prisma/issues/4146

  // Updating a many-to-many relationship using connect should update the
  // `updatedAt` field on the side where the relationship is embedded.

  val project = SchemaDsl.fromStringV11() {
     """
       | model Account {
       |   id         Int      @id
       |   tokens     Token[]
       |   updatedAt  DateTime @updatedAt
       | }
       | 
       | model Token {
       |   id           Int      @id
       |   name         String
       |   account      Account? @relation(fields: [accountId], references: [id])
       |   accountId    Int?
       |   updatedAt    DateTime @updatedAt
       | }
     """.stripMargin
  }

  "Updating a list of fields over a connect bound" should "change the update fields tagged with @updatedAt" in {
    database.setup(project)

    server.query(
      s"""mutation {
      |  createOneAccount(data: { id: 1 }) {
      |    id
      |  }
      |}
      """.stripMargin,
      project,
      legacy = false,
    )

    val updatedAt = server.query(
      s"""mutation {
      |  createOneToken(data: { id: 2, name: "test" }) {
      |    updatedAt
      |  }
      |}
      """.stripMargin,
      project,
      legacy = false,
    ).pathAsString("data.createOneToken.updatedAt")

    val tokens = server.query(
      s"""mutation {
        updateOneAccount(
          where: { id: 1 }
          data: { tokens: { connect: { id: 2 } } }
        ) {
          tokens {
            updatedAt
          }
        }
      }
      """.stripMargin,
      project,
      legacy = false,
   ).pathAsSeq("data.updateOneAccount.tokens")

   tokens(0).pathAsString("updatedAt") should not equal updatedAt
  }
}
