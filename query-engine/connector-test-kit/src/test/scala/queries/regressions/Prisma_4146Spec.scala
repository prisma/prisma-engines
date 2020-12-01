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
       |   id         String         @id @default(cuid())
       |   name       String?
       |   tokens     Token[]
       |   updatedAt  DateTime       @updatedAt
       | }
       | 
       | model Token {
       |   account      Account? @relation(fields: [accountId], references: [id])
       |   accountId    String?
       |   id           String   @id @default(cuid())
       |   updatedAt    DateTime @updatedAt
       | }
     """.stripMargin
  }

  "Updating a list of fields over a connect bound" should "change the update fields tagged with @updatedAt" taggedAs (IgnoreMsSql) in {
    val res = server.query(
      s"""mutation createOneToken{
         |createOneToken(data: {
         |    name: "a",
         |})
         |{name}
         |}
      """.stripMargin,
      project,
      legacy = false,
    )
    println(res)

    // val res2 = server.query(
    //   s"""mutation createAccount{
    //      |createOneAccount(data: {
    //      |    name: "a",
    //      |})
    //      |{name}
    //      |}
    //   """.stripMargin,
    //   project,
    //   legacy = false,
    // )
  }
}
