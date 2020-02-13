package writes.deadlocksAndTransactions

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

import scala.concurrent.Future

class VeryManyMutationsSpec extends FlatSpec with Matchers with ApiSpecBase with AwaitUtils {

  override def runOnlyForCapabilities = Set(JoinRelationLinksCapability)

  //Postgres has a limit of 32678 parameters to a query

  "The delete many Mutation" should "delete the items matching the where clause" in {
    val project: Project = SchemaDsl.fromStringV11() {
      s"""
      |model Top {
      |   id      String   @id @default(cuid())
      |   int     Int
      |   middles Middle[]
      |}
      |
      |model Middle {
      |   id  String @id @default(cuid())
      |   int Int
      |}
    """
    }
    database.setup(project)

    def createTop(int: Int): Unit = {
      val query =
        s"""mutation a {createTop(data: {
           |  int: $int
           |  middles: {create: [
           |  {int: ${int}1},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: ${int}20},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: $int},
           |  {int: ${int}40}
           |  ]}
           |}) {int}}"""

      server.query(query, project)
    }

    val futures      = (1 to 1000).map(int => Future { createTop(int) })
    val five_minutes = 300
    Future.sequence(futures).await(five_minutes)

    // relations must work with that many records
    println(server.query("""query {tops { middles { int } }}""", project))

    val update = server.query("""mutation {updateManyMiddles(where: { int_gt: 100 } data:{int: 500}){count}}""", project)
    update.pathAsLong("data.updateManyMiddles.count") should equal(36291)

    val result = server.query("""mutation {deleteManyMiddles(where: { int_gt: 100 }){count}}""", project)
    result.pathAsLong("data.deleteManyMiddles.count") should equal(36291)
  }
}
