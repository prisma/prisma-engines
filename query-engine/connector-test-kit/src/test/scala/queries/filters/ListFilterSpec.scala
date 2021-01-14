package queries.filters

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util.ConnectorTag.PostgresConnectorTag
import util._

class ListFilterSpec extends FlatSpec with Matchers with ApiSpecBase with ConnectorAwareTest {
  override def runOnlyForConnectors: Set[ConnectorTag] = Set(PostgresConnectorTag)

  val project: Project = ProjectDsl.fromString { """
     |model Test {
     |  id        String   @id @default(cuid())
     |  strList   String[]
     |  intList   Int[]
     |  floatList Float[]
     |  bIntList  BigInt[]
     |  decList   Decimal[]
     |  dtList    DateTime[]
     |  boolList  Boolean[]
     |  jsonList  Json[]
     |  bytesList Bytes[]
     |  enumList  TestEnum[]
     |}
     |
     |enum TestEnum {
     |  A
     |  B
     |}
     """.stripMargin }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)
  }

  "Queries" should "display all items if no filter is given" in {}
}
