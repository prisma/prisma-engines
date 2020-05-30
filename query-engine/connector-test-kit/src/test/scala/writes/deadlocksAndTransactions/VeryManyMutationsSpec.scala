package writes.deadlocksAndTransactions

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

import scala.concurrent.Future

class VeryManyMutationsSpec extends FlatSpec with Matchers with ApiSpecBase with AwaitUtils {

  override def runOnlyForCapabilities = Set(JoinRelationLinksCapability)

  //Postgres has a limit of 32678 parameters to a query
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
       |
       |   topId String?
       |   top   Top?    @relation(fields: [topId], references: [id])
       |}
  """
  }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
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
  }

  "Expanding relations for a lot 1000 records" should "work" in {
    // get 1000 tops and their middles
    server.query("""query {tops { middles { int } }}""", project)
  }

  "Expanding relations for 32768 records" should "work" in {
    // The Postgres communication protocol uses a signed 16 bit integer to identify the number of query parameters.
    // so you can't execute a query that contains more than 32.768 of them.
    // source: https://github.com/sfackler/rust-postgres/issues/356#issuecomment-391415848
    server.query(s"""query {middles(take: 32500) { top { int } }}""", project)
  }

  "Expanding relations for a for 40000 records" should "work" taggedAs (IgnorePostgres) in {
    // get 40000 middles and their tops
    server.query(s"""query {middles { top { int } }}""", project)
  }

  "The update many Mutation" should "work" in {
    val update = server.query("""mutation {updateManyMiddles(where: { int_gt: 100 } data:{int: 500}){count}}""", project)
    update.pathAsLong("data.updateManyMiddles.count") should equal(36291)
  }

  "The delete many Mutation" should "work" in {
    val result = server.query("""mutation {deleteManyMiddles(where: { int_gt: 100 }){count}}""", project)
    result.pathAsLong("data.deleteManyMiddles.count") should equal(36291)
  }
}
