package writes.regressions

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util.ConnectorTag.{DocumentConnectorTag, RelationalConnectorTag}
import util._

class GraphReorderRegressionSpec extends FlatSpec with Matchers with ApiSpecBase {
  // Related issue: https://github.com/prisma/prisma/issues/3081
  "The 1:1 relation checks" should "not null out the newly created nested item" in {
    val project = ProjectDsl.fromString {
      """
        |model Company {
        |  id       String    @id @default(cuid())
        |  payments Payment[]
        |}
        |
        |model Visit {
        |  id      String   @id @default(cuid())
        |  payment Payment?
        |}
        |
        |model Payment {
        |  id        Int     @id
        |  company   Company @relation(fields: [companyId], references: [id])
        |  companyId String
        |  visit     Visit?  @relation(fields: [visitId], references: [id])
        |  visitId   String?
        |}
      """.stripMargin
    }
    database.setup(project)

    // Setup
    server.query(
      """
        |mutation {
        |  createOneCompany(data:{
        |    id: "company"
        |  }) {
        |    id
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    server.query(
      """
        |mutation {
        |  createOneVisit(data:{
        |    id:"visit"
        |  }) {
        |    id
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    val result = server.query(
      """
        |mutation {
        |  updateOneVisit(
        |    where: { id: "visit" }
        |    data: { payment: { create: { id: 1, company: { connect: { id: "company" }}}}}
        |  ) {
        |    id
        |    payment {
        |      id
        |    }
        |  }
        |}
        |
      """.stripMargin,
      project,
      legacy = false
    )

    result.toString() should be("""{"data":{"updateOneVisit":{"id":"visit","payment":{"id":1}}}}""")
  }
}
