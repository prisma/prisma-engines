package queries.regressions

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

// RS: Ported
class Prisma_933Spec extends FlatSpec with Matchers with ApiSpecBase {
  // validates fix for
  //https://github.com/prisma/prisma-client-js/issues/933

  "Querying the same M:M at different levels with only the ID field" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Buyer {
         |  buyer_id Int    @id @default(autoincrement())
         |  name     String?
         |  sales    Sale[]    @relation("BuyersOnSale", references: [sale_id])
         |}
         |
         |model Sale {
         |  sale_id  Int    @id @default(autoincrement())
         |  buyers   Buyer[]   @relation("BuyersOnSale", references: [buyer_id])
         |}
         |
         |
       """.stripMargin
    }

    database.setup(project)

    server.query(
      """
       | mutation {
       |   createOneBuyer(
       |    data: {
       |      name: "Foo",
       |      sales: {
       |        create: [{}, {}]
       |      }
       |    }
       |   ) {
       |    buyer_id
       |     sales {
       |       sale_id
       |     }
       |  }
       |}
      """,
      project,
      "",
      false
    )

    val res = server.query(
      """
        | query {
        |   findManyBuyer {
        |     sales {
        |       buyers {
        |         buyer_id
        |       }
        |     }
        |   }
        | }
        |
      """, project, "", false)

    res.toString() should be("{\"data\":{\"findManyBuyer\":[{\"sales\":[{\"buyers\":[{\"buyer_id\":1}]},{\"buyers\":[{\"buyer_id\":1}]}]}]}}")
  }
}
