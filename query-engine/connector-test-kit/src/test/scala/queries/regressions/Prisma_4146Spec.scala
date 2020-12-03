package queries.regressions

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

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

  "Updating a list of fields over a connect bound" should "change the update fields tagged with @updatedAt" taggedAs (IgnoreMsSql) in {
    database.setup(project)

    server.query(
      s"""mutation createAccount{
         |createOneAccount(data: {
         |    id: 1
         |    tokens: { create: [
         |      { id: 1, name: "a" },
         |      { id: 2, name: "b" },
         |    ]}
         |})
         |{id}
         |}
      """.stripMargin,
      project,
      legacy = false,
    )

    var res = server.query(s"""query {tokens { id, name, updatedAt }}""", project)
    println(res)

    server.query(
      s"""mutation createAccount{
         |updateOneAccount(where: {
         |    id: 1
         |}, data: { account {
         |    token: { connect: {
         |      id: 1, name: "c",
         |    }}
         |}})
         |{id}
         |}
      """.stripMargin,
      project,
      legacy = false,
    )
  }
}
