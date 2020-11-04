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
        |  id        String  @id @default(cuid())
        |  company   Company @relation(fields: [companyId], references: [id])
        |  companyId String
        |  visit     Visit?  @relation(fields: [visitId], references: [id])
        |  visitId   String?
        |}
      """.stripMargin
    }
    database.setup(project)

  }
}
