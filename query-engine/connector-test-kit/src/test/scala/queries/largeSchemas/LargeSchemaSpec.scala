package queries.largeSchemas

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util.ConnectorTag.{DocumentConnectorTag, RelationalConnectorTag}
import scala.io.Source
import util._

class LargeSchemaSpec extends FlatSpec with Matchers with ApiSpecBase {

  val project = ProjectDsl.fromString {
    Source.fromFile("src/test/scala/queries/largeSchemas/large_schema.prisma").mkString
  }

  override protected def beforeEach(): Unit = {
    super.beforeEach()
    database.setup(project)
  }

  "Querying a large schema" should "succeed" in {
    val allCategories =
      s"""{
         |  findManyweekend_weekend(orderBy: { id: asc }) {
         |    id
         |  }
         |}"""

    val res1 = server.query(allCategories, project).toString
    res1 should be("""{"data":{"findManyweekend_weekend":[]}}""")
  }
}
